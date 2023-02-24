// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{atlases, CustomPipe, DrawPipe, DrawWindow, ShaderManager};
use kas::cast::*;
use kas::draw::{color::Rgba, PassId};
use kas::geom::{Quad, Rect, Vec2};
use kas::text::fonts::FaceId;
use kas::text::{Effect, Glyph, TextDisplay};
use kas::theme::RasterConfig;
use kas_text::raster::{Config, RasterGlyphImage, RasterImageFormat, SpriteDescriptor};
use rustc_hash::FxHashMap as HashMap;
use std::mem::size_of;
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorType {
    /// Single channel is alpha; color supplied by Instance
    Alpha,
    /// RGBA texture (uses image atlas)
    Rgba,
}

/// Color type, bytes per row, Size, offset, data
type RasterResult = (ColorType, u32, (u32, u32), Vec2, Vec<u8>);

/// A Sprite
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains everything needed to draw from the sprite.
#[derive(Clone, Debug)]
struct Sprite {
    color_type: ColorType,
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

impl From<Instance> for super::images::Instance {
    fn from(instance: Instance) -> Self {
        super::images::Instance {
            a: instance.a,
            b: instance.b,
            ta: instance.ta,
            tb: instance.tb,
        }
    }
}

struct Prepare {
    color_type: ColorType,
    bytes_per_row: u32,
    atlas: u32,
    origin: (u32, u32),
    size: (u32, u32),
    data: Vec<u8>,
}

/// A pipeline for rendering text
pub struct Pipeline {
    config: Config,
    atlas_pipe: atlases::Pipeline<Instance>,
    glyphs: HashMap<SpriteDescriptor, Option<Sprite>>,
    #[allow(clippy::type_complexity)]
    prepare: Vec<Prepare>,
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
            Some("text pipe"),
            bgl_common,
            512,
            wgpu::TextureFormat::R8Unorm,
            wgpu::VertexState {
                module: &shaders.vert_glyph,
                entry_point: "main",
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
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
}

impl<C: CustomPipe> DrawPipe<C> {
    /// Write to textures
    pub fn prepare_text(&mut self) {
        let text = &mut self.text;
        text.atlas_pipe.prepare(&self.device);
        self.images.atlas_pipe.prepare(&self.device);

        if !text.prepare.is_empty() {
            log::trace!("prepare: uploading {} sprites", text.prepare.len());
        }
        for p in text.prepare.drain(..) {
            let texture = match p.color_type {
                ColorType::Alpha => text.atlas_pipe.get_texture(p.atlas),
                ColorType::Rgba => self.images.atlas_pipe.get_texture(p.atlas),
            };

            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: p.origin.0,
                        y: p.origin.1,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &p.data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(p.bytes_per_row),
                    rows_per_image: NonZeroU32::new(p.size.1),
                },
                wgpu::Extent3d {
                    width: p.size.0,
                    height: p.size.1,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    /// Get a rendered sprite
    ///
    /// This returns `None` if there's nothing to render. It may also return
    /// `None` (with a warning) on error.
    fn get_glyph(&mut self, face: FaceId, dpem: f32, glyph: Glyph) -> Option<Sprite> {
        let desc = SpriteDescriptor::new(&self.text.config, face, glyph, dpem);
        if let Some(opt_sprite) = self.text.glyphs.get(&desc).cloned() {
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

        let mut result: Option<RasterResult> = None;
        if let Some(rs) = desc.raster(&self.text.config) {
            let offset = Vec2(rs.offset.0.cast(), rs.offset.1.cast());
            result = Some((ColorType::Alpha, rs.size.0, rs.size, offset, rs.data));
        } else if let Some(rs) = desc.raster_image(&self.text.config) {
            // TODO: dpem binning of SpriteDescriptor is not ideal.
            // TODO: what should our output size be? Is dpem too large?
            let dpem = desc.dpem(&self.text.config);
            match rs.format {
                #[cfg(feature = "png")]
                RasterImageFormat::PNG => result = raster_png(rs, dpem),
                #[allow(unreachable_patterns)]
                format => log::debug!("raster_glyph: unsupported format {format:?}"),
            }
        } else if let Some(_svg) = desc.svg_image() {
            println!("SVG image: {desc:?}");
        } else {
            log::debug!("raster_glyph: failed to raster {desc:?}");
        };

        let mut sprite = None;
        if let Some((color_type, bytes_per_row, size, offset, data)) = result {
            let result = match color_type {
                ColorType::Alpha => self.text.atlas_pipe.allocate(size),
                ColorType::Rgba => self.images.atlas_pipe.allocate(size),
            };
            match result {
                Ok((atlas, _, origin, tex_quad)) => {
                    let s = Sprite {
                        color_type,
                        atlas,
                        size: Vec2(size.0.cast(), size.1.cast()),
                        offset,
                        tex_quad,
                    };

                    self.text.prepare.push(Prepare {
                        color_type,
                        bytes_per_row,
                        atlas,
                        origin,
                        size,
                        data,
                    });
                    sprite = Some(s);//TODO
                }
                Err(_) => {
                    log::warn!("raster_glyph: failed to allocate glyph with size {size:?}",);
                }
            };
        }

        self.text.glyphs.insert(desc, sprite.clone());
        sprite
    }
}

#[cfg(feature = "png")]
fn raster_png(rs: RasterGlyphImage, height: f32) -> Option<RasterResult> {
    debug_assert_eq!(rs.format, RasterImageFormat::PNG);
    let offset = Vec2(rs.x.cast(), rs.y.cast());

    let mut dcdr = png::Decoder::new(rs.data);
    dcdr.set_transformations(png::Transformations::STRIP_16 | png::Transformations::EXPAND);
    let mut rdr = dcdr
        .read_info()
        .map_err(|e| log::warn!("raster_glyph: {e:?}"))
        .ok()?;

    let info = rdr.info();
    if info.animation_control.is_some() {
        log::warn!("raster_glyph: animation not supported");
    }

    let mut buf = vec![0; rdr.output_buffer_size()];
    let info = rdr
        .next_frame(&mut buf)
        .map_err(|e| log::warn!("raster_glyph: {e:?}"))
        .ok()?;
    debug_assert!(info.width == rs.width as u32 && info.height == rs.height as u32);
    let size = (info.width, info.height);
    let mut resize = None;
    if info.height != height.cast_nearest() {
        println!("TODO: downscale {size:?} to {height}");
        let w = f32::conv(size.0) / f32::conv(size.1) * height;
        resize = Some((u32::conv_nearest(w), u32::conv_nearest(height)));
    }
    debug_assert_eq!(info.bit_depth, png::BitDepth::Eight); // via transformations
    match info.color_type {
        png::ColorType::Grayscale => {
            let color_type = ColorType::Alpha; // possible mis-use
            if let Some(size) = resize {
                buf = vec![127; usize::conv(size.0) * usize::conv(size.1)];
                Some((color_type, size.0, size, offset, buf))
            } else {
                Some((color_type, info.line_size.cast(), size, offset, buf))
            }
        }
        png::ColorType::Rgba => {
            let color_type = ColorType::Rgba;
            if let Some(size) = resize {
                buf = vec![127; 4 * usize::conv(size.0) * usize::conv(size.1)];
                Some((color_type, 4 * size.0, size, offset, buf))
            } else {
                Some((color_type, info.line_size.cast(), size, offset, buf))
            }
        }
        color => {
            log::warn!("raster_glyph: unsupported color type {color:?}");
            None
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
}

impl<C: CustomPipe> DrawPipe<C> {
    pub fn draw_text_impl(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
    ) {
        let rect = Quad::conv(rect);

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph| {
            if let Some(sprite) = self.get_glyph(face, dpem, glyph) {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    match sprite.color_type {
                        ColorType::Alpha => window.text.atlas.rect(pass, sprite.atlas, instance),
                        ColorType::Rgba => window.images.atlas.rect(pass, sprite.atlas, instance.into()),
                    }
                }
            }
        };
        if let Err(e) = text.glyphs(for_glyph) {
            log::warn!("Window: display failed: {e}");
        }
    }

    pub fn draw_text_effects_impl(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) {
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .get(0)
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            self.draw_text_impl(window, pass, rect, text, col);
            return;
        }

        let rect = Quad::conv(rect);

        let mut for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _: usize, _: ()| {
            if let Some(sprite) = self.get_glyph(face, dpem, glyph) {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    match sprite.color_type {
                        ColorType::Alpha => window.text.atlas.rect(pass, sprite.atlas, instance),
                        ColorType::Rgba => window.images.atlas.rect(pass, sprite.atlas, instance.into()),
                    }
                }
            }
        };

        let result = if effects.len() > 1
            || effects
                .get(0)
                .map(|e| *e != Effect::default(()))
                .unwrap_or(false)
        {
            let for_rect = |x1, x2, y: f32, h: f32, _, _| {
                let y = y.ceil();
                let y2 = y + h.ceil();
                if let Some(quad) = Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2))
                    .intersection(&rect)
                {
                    window.shaded_square.rect(pass, quad, col);
                }
            };
            text.glyphs_with_effects(effects, (), for_glyph, for_rect)
        } else {
            text.glyphs(|face, dpem, glyph| for_glyph(face, dpem, glyph, 0, ()))
        };

        if let Err(e) = result {
            log::warn!("Window: display failed: {e}");
        }
    }

    pub fn draw_text_effects_rgba_impl(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    ) {
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .get(0)
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            let col = effects.get(0).map(|e| e.aux).unwrap_or(Rgba::BLACK);
            self.draw_text_impl(window, pass, rect, text, col);
            return;
        }

        let rect = Quad::conv(rect);

        let for_glyph = |face: FaceId, dpem: f32, glyph: Glyph, _, col: Rgba| {
            if let Some(sprite) = self.get_glyph(face, dpem, glyph) {
                let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                let b = a + sprite.size;
                if let Some(instance) = Instance::new(rect, a, b, sprite.tex_quad, col) {
                    match sprite.color_type {
                        ColorType::Alpha => window.text.atlas.rect(pass, sprite.atlas, instance),
                        ColorType::Rgba => window.images.atlas.rect(pass, sprite.atlas, instance.into()),
                    }
                }
            }
        };

        let for_rect = |x1, x2, y: f32, h: f32, _, col: Rgba| {
            let y = y.ceil();
            let y2 = y + h.ceil();
            if let Some(quad) =
                Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2)).intersection(&rect)
            {
                window.shaded_square.rect(pass, quad, col);
            }
        };

        if let Err(e) = text.glyphs_with_effects(effects, Rgba::BLACK, for_glyph, for_rect) {
            log::warn!("Window: display failed: {e}");
        }
    }
}
