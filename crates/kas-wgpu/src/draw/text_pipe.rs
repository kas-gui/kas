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
use kas_text::fonts::{self, FaceId};
use kas_text::{Effect, Glyph, GlyphId, TextDisplay};
use rustc_hash::FxHashMap as HashMap;
use std::mem::size_of;

kas::impl_scope! {
    /// Raster configuration
    #[derive(Debug, PartialEq)]
    #[impl_default]
    pub struct Config {
        scale_steps: f32 = 4.0,
        subpixel_threshold: f32 = 18.0,
        subpixel_steps: u8 = 5,
    }
}

enum Rasterer {
    #[cfg(feature = "ab_glyph")]
    AbGlyph,
    Swash,
}

impl Default for Rasterer {
    #[allow(clippy::needless_return, unreachable_code)]
    fn default() -> Self {
        return Rasterer::Swash;
    }
}

/// Configuration read/write/format errors
#[derive(thiserror::Error, Debug)]
pub enum RasterError {
    #[error("allocation failed")]
    Alloc(#[from] AllocError),
    #[error("failed")]
    Failed,
    #[allow(unused)]
    #[error("zero-sized")]
    Zero,
}

/// A Sprite descriptor
///
/// This descriptor includes all important properties of a rastered glyph in a
/// small, easily hashable value. It is thus ideal for caching rastered glyphs
/// in a `HashMap`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct SpriteDescriptor(u64);

impl std::fmt::Debug for SpriteDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dpem_steps = ((self.0 & 0x00FF_FFFF_0000_0000) >> 32) as u32;
        let x_steps = ((self.0 & 0x0F00_0000_0000_0000) >> 56) as u8;
        let y_steps = ((self.0 & 0xF000_0000_0000_0000) >> 60) as u8;
        f.debug_struct("SpriteDescriptor")
            .field("face", &self.face())
            .field("glyph", &self.glyph())
            .field("dpem_steps", &dpem_steps)
            .field("offset_steps", &(x_steps, y_steps))
            .finish()
    }
}

impl SpriteDescriptor {
    /// Choose a sub-pixel precision multiplier based on scale (pixels per Em)
    ///
    /// Must return an integer between 1 and 16.
    fn sub_pixel_from_dpem(config: &Config, dpem: f32) -> u8 {
        if dpem < config.subpixel_threshold {
            config.subpixel_steps
        } else {
            1
        }
    }

    /// Construct
    ///
    /// Most parameters come from [`TextDisplay::glyphs`] output. See also [`raster`].
    pub fn new(config: &Config, face: FaceId, glyph: Glyph, dpem: f32) -> Self {
        let face: u16 = face.get().cast();
        let glyph_id: u16 = glyph.id.0;
        let steps = Self::sub_pixel_from_dpem(config, dpem);
        let mult = f32::conv(steps);
        let dpem = u32::conv_trunc(dpem * config.scale_steps + 0.5);
        let x_off = u8::conv_trunc(glyph.position.0.fract() * mult) % steps;
        let y_off = u8::conv_trunc(glyph.position.1.fract() * mult) % steps;
        assert!(dpem & 0xFF00_0000 == 0 && x_off & 0xF0 == 0 && y_off & 0xF0 == 0);
        let packed = face as u64
            | ((glyph_id as u64) << 16)
            | ((dpem as u64) << 32)
            | ((x_off as u64) << 56)
            | ((y_off as u64) << 60);
        SpriteDescriptor(packed)
    }

    /// Get `FaceId` descriptor
    pub fn face(self) -> FaceId {
        FaceId::from((self.0 & 0x0000_0000_0000_FFFF) as u32)
    }

    /// Get `GlyphId` descriptor
    pub fn glyph(self) -> GlyphId {
        GlyphId(((self.0 & 0x0000_0000_FFFF_0000) >> 16).cast())
    }

    /// Get scale (pixels per Em)
    pub fn dpem(self, config: &Config) -> f32 {
        let dpem_steps = ((self.0 & 0x00FF_FFFF_0000_0000) >> 32) as u32;
        f32::conv(dpem_steps) / config.scale_steps
    }

    /// Get fractional position
    ///
    /// This may optionally be used (depending on [`Config`]) to improve letter
    /// spacing at small font sizes. Returns the `(x, y)` offsets in the range
    /// `0.0 ≤ x < 1.0` (and the same for `y`).
    pub fn fractional_position(self, config: &Config) -> (f32, f32) {
        let mult = 1.0 / f32::conv(Self::sub_pixel_from_dpem(config, self.dpem(config)));
        let x_steps = ((self.0 & 0x0F00_0000_0000_0000) >> 56) as u8;
        let y_steps = ((self.0 & 0xF000_0000_0000_0000) >> 60) as u8;
        let x = f32::conv(x_steps) * mult;
        let y = f32::conv(y_steps) * mult;
        (x, y)
    }
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
    rasterer: Rasterer,
    #[allow(unused)]
    sb_align: bool,
    #[allow(unused)]
    hint: bool,
    config: Config,
    atlas_pipe: atlases::Pipeline<Instance>,
    glyphs: HashMap<SpriteDescriptor, Sprite>,
    #[allow(clippy::type_complexity)]
    prepare: Vec<(u32, (u32, u32), (u32, u32), Vec<u8>)>,
    scale_cx: swash::scale::ScaleContext,
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
            rasterer: Default::default(),
            sb_align: false,
            hint: false,
            config: Config::default(),
            atlas_pipe,
            glyphs: Default::default(),
            prepare: Default::default(),
            scale_cx: Default::default(),
        }
    }

    pub fn set_raster_config(&mut self, config: &RasterConfig) {
        match config.mode {
            #[cfg(feature = "ab_glyph")]
            0 | 1 => self.rasterer = Rasterer::AbGlyph,
            3 | 4 => self.rasterer = Rasterer::Swash,
            x => log::warn!("raster mode {x} unavailable; falling back to default"),
        };

        self.sb_align = config.mode == 1;
        self.hint = config.mode == 4;

        self.config = Config {
            scale_steps: config.scale_steps.cast(),
            subpixel_threshold: config.subpixel_threshold.cast(),
            subpixel_steps: config.subpixel_steps.clamp(1, 16),
        };

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

        let result = match self.rasterer {
            #[cfg(feature = "ab_glyph")]
            Rasterer::AbGlyph => self.raster_ab_glyph(desc),
            Rasterer::Swash => self.raster_swash(desc),
        };

        let sprite = match result {
            Ok(sprite) => sprite,
            Err(RasterError::Zero) => {
                // Ignore this common error
                Sprite::default()
            }
            Err(err) => {
                log::warn!("raster_glyph failed: {err}");
                Sprite::default()
            }
        };

        self.glyphs.insert(desc, sprite.clone());
        sprite
    }

    #[cfg(feature = "ab_glyph")]
    fn raster_ab_glyph(&mut self, desc: SpriteDescriptor) -> Result<Sprite, RasterError> {
        use ab_glyph::Font;

        let id = desc.glyph();
        let face = desc.face();
        let face_store = fonts::library().get_face_store(face);
        let dpem = desc.dpem(&self.config);

        let (mut x, y) = desc.fractional_position(&self.config);
        if self.sb_align && desc.dpem(&self.config) >= self.config.subpixel_threshold {
            let sf = face_store.face_ref().scale_by_dpem(dpem);
            x -= sf.h_side_bearing(id);
        }

        let font = face_store.ab_glyph();
        let scale = dpem * font.height_unscaled() / font.units_per_em().unwrap();
        let glyph = ab_glyph::Glyph {
            id: ab_glyph::GlyphId(id.0),
            scale: scale.into(),
            position: ab_glyph::point(x, y),
        };
        let outline = font.outline_glyph(glyph).ok_or(RasterError::Failed)?;

        let bounds = outline.px_bounds();
        let offset: (i32, i32) = (bounds.min.x.cast_trunc(), bounds.min.y.cast_trunc());
        let size = bounds.max - bounds.min;
        let size = (u32::conv_trunc(size.x), u32::conv_trunc(size.y));
        if size.0 == 0 || size.1 == 0 {
            return Err(RasterError::Zero);
        }

        let mut data = vec![0; usize::conv(size.0 * size.1)];
        outline.draw(|x, y, c| {
            // Convert to u8 with saturating conversion, rounding down:
            data[usize::conv((y * size.0) + x)] = (c * 256.0) as u8;
        });

        let (atlas, _, origin, tex_quad) = self.atlas_pipe.allocate(size)?;

        self.prepare.push((atlas, origin, size, data));

        Ok(Sprite {
            atlas,
            valid: true,
            size: Vec2(size.0.cast(), size.1.cast()),
            offset: Vec2(offset.0.cast(), offset.1.cast()),
            tex_quad,
        })
    }

    fn raster_swash(&mut self, desc: SpriteDescriptor) -> Result<Sprite, RasterError> {
        use swash::scale::{image::Content, Render, Source, StrikeWith};
        use swash::zeno::Format;

        // TODO: we should re-use scaler when rendering glyphs for a layout/run
        let font = fonts::library().get_face_store(desc.face()).swash();
        let mut scaler = self
            .scale_cx
            .builder(font)
            .size(desc.dpem(&self.config))
            .hint(self.hint)
            .build();

        let sources = &[
            // TODO: Support coloured rendering? These can replace Source::Bitmap
            // Source::ColorOutline(0),
            // Source::ColorBitmap(StrikeWith::BestFit),
            Source::Bitmap(StrikeWith::BestFit),
            Source::Outline,
        ];

        let image = Render::new(sources)
            .format(Format::Alpha)
            .offset(desc.fractional_position(&self.config).into())
            .render(&mut scaler, desc.glyph().0.into())
            .ok_or(RasterError::Failed)?;

        let offset = (image.placement.left, -image.placement.top);
        let size = (image.placement.width, image.placement.height);
        if size.0 == 0 || size.1 == 0 {
            return Err(RasterError::Zero);
        }

        match image.content {
            Content::Mask => {
                let (atlas, _, origin, tex_quad) = self.atlas_pipe.allocate(size)?;

                self.prepare.push((atlas, origin, size, image.data));

                Ok(Sprite {
                    atlas,
                    valid: true,
                    size: Vec2(size.0.cast(), size.1.cast()),
                    offset: Vec2(offset.0.cast(), offset.1.cast()),
                    tex_quad,
                })
            }
            Content::SubpixelMask => unimplemented!(),
            Content::Color => unimplemented!(),
        }
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
