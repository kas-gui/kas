// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{atlases, ShaderManager};
use kas::cast::traits::*;
use kas::config::RasterConfig;
use kas::draw::{color::Rgba, AllocError, PassId};
use kas::geom::{Quad, Rect, Vec2};
use kas::text::fonts::FaceId;
use kas::text::{Effect, Glyph, TextDisplay};
use kas_text::raster::{raster, Config, SpriteDescriptor};
use rustc_hash::FxHashMap as HashMap;
use std::mem::size_of;

/// Configuration read/write/format errors
#[derive(thiserror::Error, Debug)]
pub enum RasterError {
    #[error("allocation failed")]
    Alloc(#[from] AllocError),
    #[error("failed")]
    Failed,
}

/// A Sprite
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains everything needed to draw from the sprite.
#[derive(Clone, Debug, Default)]
struct Sprite {
    atlas: u32,
    // TODO(opt): u16 or maybe even u8 would be enough
    valid: bool,
    size: Vec2,
    offset: Vec2,
    tex_quad: Quad,
}

impl Sprite {
    /// Is this a valid sprite or a non-renderable placeholder?
    fn is_valid(&self) -> bool {
        self.valid
    }
}

/// Screen and texture coordinates
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    a: Vec2,
    b: Vec2,
    ta: Vec2,
    tb: Vec2,
    col: Rgba,
}
unsafe impl bytemuck::Zeroable for Instance {}
unsafe impl bytemuck::Pod for Instance {}

impl Instance {
    /// Construct, clamping to rect
    // TODO(opt): should this use a buffer? Should TextDisplay::prepare_lines prune glyphs?
    fn new(rect: Quad, mut a: Vec2, mut b: Vec2, tex_quad: Quad, col: Rgba) -> Option<Self> {
        if !(a.0 < rect.b.0 && a.1 < rect.b.1 && b.0 > rect.a.0 && b.1 > rect.a.1) {
            return None;
        }

        let (mut ta, mut tb) = (tex_quad.a, tex_quad.b);
        if !a.ge(rect.a) || !b.le(rect.b) {
            let size_inv = 1.0 / (b - a);
            let fa0 = 0f32.max((rect.a.0 - a.0) * size_inv.0);
            let fa1 = 0f32.max((rect.a.1 - a.1) * size_inv.1);
            let fb0 = 1f32.min((rect.b.0 - a.0) * size_inv.0);
            let fb1 = 1f32.min((rect.b.1 - a.1) * size_inv.1);

            let ts = tb - ta;
            tb = ta + ts * Vec2(fb0, fb1);
            ta += ts * Vec2(fa0, fa1);

            a.0 = a.0.clamp(rect.a.0, rect.b.0);
            a.1 = a.1.clamp(rect.a.1, rect.b.1);
            b.0 = b.0.clamp(rect.a.0, rect.b.0);
            b.1 = b.1.clamp(rect.a.1, rect.b.1);
        }

        Some(Instance { a, b, ta, tb, col })
    }
}

/// A pipeline for rendering text
pub struct Pipeline {
    config: Config,
    atlas_pipe: atlases::Pipeline<Instance>,
    glyphs: HashMap<SpriteDescriptor, Sprite>,
    #[allow(clippy::type_complexity)]
    prepare: Vec<(u32, (u32, u32), (u32, u32), Vec<u8>)>,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        shaders: &ShaderManager,
        bgl_common: &wgpu::BindGroupLayout,
    ) -> Self {
        let atlas_pipe = atlases::Pipeline::new(
            device,
            Some("text pipe"),
            bgl_common,
            512,
            wgpu::TextureFormat::R8Unorm,
            wgpu::VertexState {
                module: &shaders.vert_glyph,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x2,
                        3 => Float32x2,
                        4 => Float32x4,
                    ],
                }],
            },
            wgpu::FragmentState {
                module: &shaders.frag_glyph,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            },
        );
        Pipeline {
            config: Config::default(),
            atlas_pipe,
            glyphs: Default::default(),
            prepare: Default::default(),
        }
    }

    pub fn set_raster_config(&mut self, config: &RasterConfig) {
        self.config = Config::new(
            config.mode,
            config.scale_steps,
            config.subpixel_threshold,
            config.subpixel_steps,
        )
        // NOTE: possibly this should force re-drawing of all glyphs, but for
        // now that is out of scope
    }

    /// Write to textures
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.atlas_pipe.prepare(device);

        if !self.prepare.is_empty() {
            log::trace!("prepare: uploading {} sprites", self.prepare.len());
        }
        for (atlas, origin, size, data) in self.prepare.drain(..) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: self.atlas_pipe.get_texture(atlas),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: origin.0,
                        y: origin.1,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.0),
                    rows_per_image: Some(size.1),
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        self.atlas_pipe
            .render(&window.atlas, pass, rpass, bg_common);
    }

    /// Get a rendered sprite
    ///
    /// The result may not be valid; see [`Sprite::is_valid`].
    fn get_glyph(&mut self, face: FaceId, dpem: f32, glyph: Glyph) -> Sprite {
        let desc = SpriteDescriptor::new(&self.config, face, glyph, dpem);
        if let Some(sprite) = self.glyphs.get(&desc).cloned() {
            sprite
        } else {
            // NOTE: this branch is *rare*. We don't use HashMap::entry and push
            // rastering to another function to optimise for the common case.
            self.raster_glyph(desc)
        }
    }

    fn raster_glyph(&mut self, desc: SpriteDescriptor) -> Sprite {
        // NOTE: we only need the allocation and coordinates now; the
        // rendering could be offloaded (though this may not be useful).

        let sprite = match self.raster_kas_text_glyph(desc) {
            Ok(sprite) => sprite,
            Err(err) => {
                log::warn!("raster_glyph failed: {err}");
                Sprite::default()
            }
        };

        self.glyphs.insert(desc, sprite.clone());
        sprite
    }

    fn raster_kas_text_glyph(&mut self, desc: SpriteDescriptor) -> Result<Sprite, RasterError> {
        let rs = raster(&self.config, desc).ok_or(RasterError::Failed)?;
        let (atlas, _, origin, tex_quad) = self.atlas_pipe.allocate(rs.size)?;

        self.prepare.push((atlas, origin, rs.size, rs.data));

        Ok(Sprite {
            atlas,
            valid: true,
            size: Vec2(rs.size.0.cast(), rs.size.1.cast()),
            offset: Vec2(rs.offset.0.cast(), rs.offset.1.cast()),
            tex_quad,
        })
    }
}

/// Per-window state
#[derive(Debug, Default)]
pub struct Window {
    atlas: atlases::Window<Instance>,
}

impl Window {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        self.atlas.write_buffers(device, staging_belt, encoder);
    }

    pub fn text(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
    ) {
        let rect = Quad::conv(rect);

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph| {
            let sprite = pipe.get_glyph(face, dpem, glyph);
            if sprite.is_valid() {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    self.atlas.rect(pass, sprite.atlas, instance);
                }
            }
        };
        text.glyphs(for_glyph);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_effects(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
        mut draw_quad: impl FnMut(Quad),
    ) {
        let rect = Quad::conv(rect);

        let mut for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _: usize, _: ()| {
            let sprite = pipe.get_glyph(face, dpem, glyph);
            if sprite.is_valid() {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    self.atlas.rect(pass, sprite.atlas, instance);
                }
            }
        };

        if effects.len() > 1
            || effects
                .first()
                .map(|e| *e != Effect::default(()))
                .unwrap_or(false)
        {
            let for_rect = |x1, x2, y: f32, h: f32, _, _| {
                let y = y.ceil();
                let y2 = y + h.ceil();
                if let Some(quad) = Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2))
                    .intersection(&rect)
                {
                    draw_quad(quad);
                }
            };
            text.glyphs_with_effects(effects, (), for_glyph, for_rect)
        } else {
            text.glyphs(|face, dpem, glyph| for_glyph(face, dpem, glyph, 0, ()))
        };
    }

    pub fn text_effects_rgba(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
        mut draw_quad: impl FnMut(Quad, Rgba),
    ) {
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .first()
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            let col = effects.first().map(|e| e.aux).unwrap_or(Rgba::BLACK);
            self.text(pipe, pass, rect, text, col);
            return;
        }

        let rect = Quad::conv(rect);

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _, col: Rgba| {
            let sprite = pipe.get_glyph(face, dpem, glyph);
            if sprite.is_valid() {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    self.atlas.rect(pass, sprite.atlas, instance);
                }
            }
        };

        let for_rect = |x1, x2, y: f32, h: f32, _, col: Rgba| {
            let y = y.ceil();
            let y2 = y + h.ceil();
            if let Some(quad) =
                Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2)).intersection(&rect)
            {
                draw_quad(quad, col);
            }
        };

        text.glyphs_with_effects(effects, Rgba::BLACK, for_glyph, for_rect)
    }
}
