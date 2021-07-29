// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{atlases, ShaderManager};
use kas::cast::*;
use kas::draw::{color::Rgba, PassId};
use kas::geom::{Quad, Vec2};
use kas::text::fonts::FaceId;
use kas::text::{Effect, Glyph, TextDisplay};
use kas_text::raster::{raster, Config, SpriteDescriptor};
use kas_theme::RasterConfig;
use rustc_hash::FxHashMap as HashMap;
use std::mem::size_of;
use std::num::NonZeroU32;

/// A Sprite
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains everything needed to draw from the sprite.
#[derive(Clone, Debug)]
struct Sprite {
    atlas: u32,
    // TODO(opt): u16 or maybe even u8 would be enough
    size: Vec2,
    offset: Vec2,
    tex_quad: Quad,
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

/// A pipeline for rendering text
pub struct Pipeline {
    config: Config,
    atlas_pipe: atlases::Pipeline<Instance>,
    glyphs: HashMap<SpriteDescriptor, Option<Sprite>>,
    #[allow(clippy::type_complexity)]
    prepare: Vec<(u32, (u32, u32), (u32, u32), Vec<u8>)>,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        shaders: &ShaderManager,
        bgl_common: &wgpu::BindGroupLayout,
        config: &RasterConfig,
    ) -> Self {
        let atlas_pipe = atlases::Pipeline::new(
            device,
            bgl_common,
            512,
            wgpu::TextureFormat::R8Unorm,
            wgpu::VertexState {
                module: &shaders.vert_glyph,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
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
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            },
        );
        Pipeline {
            config: Config::new(
                config.mode,
                config.scale_steps,
                config.subpixel_threshold,
                config.subpixel_steps,
            ),
            atlas_pipe,
            glyphs: Default::default(),
            prepare: Default::default(),
        }
    }

    /// Write to textures
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.atlas_pipe.prepare(device);

        if !self.prepare.is_empty() {
            log::trace!(
                "Pipeline::prepare: uploading {} sprites",
                self.prepare.len()
            );
        }
        for (atlas, origin, size, data) in self.prepare.drain(..) {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: self.atlas_pipe.get_texture(atlas),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: origin.0,
                        y: origin.1,
                        z: 0,
                    },
                },
                &data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(size.0),
                    rows_per_image: NonZeroU32::new(size.1),
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
    /// This returns `None` if there's nothing to render. It may also return
    /// `None` (with a warning) on error.
    fn get_glyph(&mut self, face: FaceId, dpem: f32, glyph: Glyph) -> Option<Sprite> {
        let desc = SpriteDescriptor::new(&self.config, face, glyph, dpem);
        if let Some(opt_sprite) = self.glyphs.get(&desc).cloned() {
            opt_sprite
        } else {
            // NOTE: this branch is *rare*. We don't use HashMap::entry and push
            // rastering to another function to optimise for the common case.
            self.raster_glyph(desc)
        }
    }

    fn raster_glyph(&mut self, desc: SpriteDescriptor) -> Option<Sprite> {
        // NOTE: we only need the allocation and coordinates now; the
        // rendering could be offloaded (though this may not be useful).
        let mut sprite = None;
        if let Some(rs) = raster(&self.config, desc) {
            match self.atlas_pipe.allocate(rs.size) {
                Ok((atlas, _, origin, tex_quad)) => {
                    let s = Sprite {
                        atlas,
                        size: Vec2(rs.size.0.cast(), rs.size.1.cast()),
                        offset: Vec2(rs.offset.0.cast(), rs.offset.1.cast()),
                        tex_quad,
                    };

                    self.prepare.push((s.atlas, origin, rs.size, rs.data));
                    sprite = Some(s);
                }
                Err(_) => {
                    log::warn!(
                        "text_pipe: failed to allocate glyph with size {:?}",
                        rs.size
                    );
                }
            };
        } else {
            // This comes up a lot and is usually harmless
            log::debug!(
                "Failed to raster glyph {:?} of face {:?}",
                desc.glyph(),
                desc.face()
            );
        };

        self.glyphs.insert(desc, sprite.clone());
        sprite
    }
}

/// Per-window state
#[derive(Debug, Default)]
pub struct Window {
    atlas: atlases::Window<Instance>,
    duration: std::time::Duration,
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

    /// Get microseconds used for text during since last call
    pub fn dur_micros(&mut self) -> u128 {
        let micros = self.duration.as_micros();
        self.duration = Default::default();
        micros
    }

    pub fn text(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
    ) {
        let pos = pos.round();
        let time = std::time::Instant::now();

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph| {
            if let Some(sprite) = pipe.get_glyph(face, dpem, glyph) {
                let pos = pos + Vec2::from(glyph.position).floor();
                let a = pos + sprite.offset;
                let b = a + sprite.size;
                let (ta, tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
                let instance = Instance { a, b, ta, tb, col };
                // TODO(opt): avoid calling repeatedly?
                self.atlas.rect(pass, sprite.atlas, instance);
            }
        };
        text.glyphs(for_glyph);

        self.duration += time.elapsed();
    }

    pub fn text_col_effects(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) -> Vec<Quad> {
        let pos = pos.round();
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .get(0)
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            self.text(pipe, pass, pos, text, col);
            return vec![];
        }

        let time = std::time::Instant::now();
        let mut rects = vec![];

        let mut for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _: usize, _: ()| {
            if let Some(sprite) = pipe.get_glyph(face, dpem, glyph) {
                let pos = pos + Vec2::from(glyph.position).floor();
                let a = pos + sprite.offset;
                let b = a + sprite.size;
                let (ta, tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
                let instance = Instance { a, b, ta, tb, col };
                // TODO(opt): avoid calling repeatedly?
                self.atlas.rect(pass, sprite.atlas, instance);
            }
        };

        if effects.len() > 1
            || effects
                .get(0)
                .map(|e| *e != Effect::default(()))
                .unwrap_or(false)
        {
            let for_rect = |x1, x2, y: f32, h: f32, _, _| {
                let y = y.ceil();
                let y2 = y + h.ceil();
                let quad = Quad::with_coords(pos + Vec2(x1, y), pos + Vec2(x2, y2));
                rects.push(quad);
            };
            text.glyphs_with_effects(effects, (), for_glyph, for_rect);
        } else {
            text.glyphs(|face, dpem, glyph| for_glyph(face, dpem, glyph, 0, ()));
        }

        self.duration += time.elapsed();
        rects
    }

    pub fn text_effects(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    ) -> Vec<(Quad, Rgba)> {
        let pos = pos.round();
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .get(0)
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            let col = effects.get(0).map(|e| e.aux).unwrap_or(Rgba::BLACK);
            self.text(pipe, pass, pos, text, col);
            return vec![];
        }

        let time = std::time::Instant::now();
        let mut rects = vec![];

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _, col: Rgba| {
            if let Some(sprite) = pipe.get_glyph(face, dpem, glyph) {
                let pos = pos + Vec2::from(glyph.position).floor();
                let a = pos + sprite.offset;
                let b = a + sprite.size;
                let (ta, tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
                let instance = Instance { a, b, ta, tb, col };
                // TODO(opt): avoid calling repeatedly?
                self.atlas.rect(pass, sprite.atlas, instance);
            }
        };

        let for_rect = |x1, x2, y: f32, h: f32, _, col: Rgba| {
            let y = y.ceil();
            let y2 = y + h.ceil();
            let quad = Quad::with_coords(pos + Vec2(x1, y), pos + Vec2(x2, y2));
            rects.push((quad, col));
        };

        text.glyphs_with_effects(effects, Rgba::BLACK, for_glyph, for_rect);

        self.duration += time.elapsed();
        rects
    }
}
