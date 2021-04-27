// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{atlases, Rgba, ShaderManager};
use ab_glyph::{Font, FontRef};
use kas::cast::*;
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Vec2};
use kas::text::fonts::{fonts, FontId};
use kas::text::{Effect, Glyph, TextDisplay};
use std::collections::hash_map::{Entry, HashMap};
use std::mem::size_of;

fn to_vec2(p: ab_glyph::Point) -> Vec2 {
    Vec2(p.x, p.y)
}

/// Scale multiplier for fixed-precision
///
/// This should be an integer `n >= 1`, e.g. `n = 4` provides four sub-pixel
/// steps of precision. It is also required that `n * h < (1 << 24)` where
/// `h` is the text height in pixels.
const SCALE_MULT: f32 = 4.0;

/// A Sprite descriptor
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains those properties.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct SpriteDescriptor(u64);

impl SpriteDescriptor {
    /// Choose a sub-pixel precision multiplier based on the height
    ///
    /// Must return an integer between 1 and 15.
    fn sub_pixel_from_height(height: f32) -> f32 {
        // Due to rounding sub-pixel precision is disabled for height > 20
        (30.0 / height).round().clamp(1.0, 15.0)
    }

    fn new(font: FontId, glyph: Glyph, height: f32) -> Self {
        let font: u16 = font.get().cast();
        let glyph_id: u16 = glyph.id.0;
        let mult = Self::sub_pixel_from_height(height);
        let height: u32 = (height * SCALE_MULT).cast_nearest();
        let x_off: u8 = (glyph.position.0.fract() * mult).cast_nearest();
        let y_off: u8 = (glyph.position.1.fract() * mult).cast_nearest();
        assert!(height & 0xFF00_0000 == 0 && x_off & 0xF0 == 0 && y_off & 0xF0 == 0);
        let packed = font as u64
            | ((glyph_id as u64) << 16)
            | ((height as u64) << 32)
            | ((x_off as u64) << 56)
            | ((y_off as u64) << 60);
        SpriteDescriptor(packed)
    }

    fn font(self) -> usize {
        (self.0 & 0x0000_0000_0000_FFFF) as usize
    }

    fn glyph(self) -> u16 {
        ((self.0 & 0x0000_0000_FFFF_0000) >> 16) as u16
    }

    fn height(self) -> f32 {
        let height = ((self.0 & 0x00FF_FFFF_0000_0000) >> 32) as u32;
        f32::conv(height) / SCALE_MULT
    }

    fn fractional_position(self) -> (f32, f32) {
        let mult = 1.0 / Self::sub_pixel_from_height(self.height());
        let x = ((self.0 & 0x0F00_0000_0000_0000) >> 56) as u8;
        let y = ((self.0 & 0xF000_0000_0000_0000) >> 60) as u8;
        let x = f32::conv(x) * mult;
        let y = f32::conv(y) * mult;
        (x, y)
    }
}

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

impl atlases::Pipeline<Instance> {
    fn rasterize(
        &mut self,
        font: &FontRef<'static>,
        desc: SpriteDescriptor,
    ) -> Option<(Sprite, (u32, u32), (u32, u32), Vec<u8>)> {
        let fract_pos = desc.fractional_position();
        let glyph = ab_glyph::Glyph {
            id: ab_glyph::GlyphId(desc.glyph()),
            scale: desc.height().into(),
            position: fract_pos.into(),
        };
        let outline = font.outline_glyph(glyph)?;

        let bounds = outline.px_bounds();
        let size = to_vec2(bounds.max - bounds.min);
        let offset = to_vec2(bounds.min) - Vec2(fract_pos.0.round(), fract_pos.1.round());
        let size_u32 = (u32::conv_trunc(size.0), u32::conv_trunc(size.1));

        let (atlas, _, origin, tex_quad) = match self.allocate(size_u32) {
            Ok(result) => result,
            Err(_) => {
                log::warn!(
                    "text_pipe: failed to allocate glyph with size {:?}",
                    size_u32
                );
                return None;
            }
        };

        let mut data = Vec::new();
        data.resize(usize::conv(size_u32.0 * size_u32.1), 0u8);
        outline.draw(|x, y, c| {
            // Convert to u8 with saturating conversion, rounding down:
            data[usize::conv((y * size_u32.0) + x)] = (c * 256.0) as u8;
        });

        let sprite = Sprite {
            atlas,
            size,
            offset,
            tex_quad,
        };

        Some((sprite, origin, size_u32, data))
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

/// A pipeline for rendering text
pub struct Pipeline {
    atlas_pipe: atlases::Pipeline<Instance>,
    fonts: Vec<FontRef<'static>>,
    glyphs: HashMap<SpriteDescriptor, Option<Sprite>>,
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
            &bgl_common,
            512,
            wgpu::TextureFormat::R8Unorm,
            wgpu::VertexState {
                module: &shaders.vert_glyph,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float2,
                        1 => Float2,
                        2 => Float2,
                        3 => Float2,
                        4 => Float4,
                    ],
                }],
            },
            &shaders.frag_glyph,
        );
        Pipeline {
            atlas_pipe,
            fonts: Default::default(),
            glyphs: Default::default(),
            prepare: Default::default(),
        }
    }

    /// Prepare fonts
    ///
    /// This must happen before any drawing is queued. TODO: perhaps instead
    /// use temporary IDs for unrastered glyphs and update in `prepare`?
    pub fn prepare_fonts(&mut self) {
        let fonts = fonts();
        let n1 = self.fonts.len();
        let n2 = fonts.num_fonts();
        if n2 > n1 {
            let font_data = fonts.font_data();
            for i in n1..n2 {
                let (data, index) = font_data.get_data(i);
                let font = FontRef::try_from_slice_and_index(data, index).unwrap();
                self.fonts.push(font);
            }
        }
    }

    /// Write to textures
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.atlas_pipe.prepare(device);

        log::trace!("Pipeline::prepare: {} sprites", self.prepare.len());
        for (atlas, origin, size, data) in self.prepare.drain(..) {
            queue.write_texture(
                wgpu::TextureCopyView {
                    texture: self.atlas_pipe.get_texture(atlas),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: origin.0,
                        y: origin.1,
                        z: 0,
                    },
                },
                &data,
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: size.0,
                    rows_per_image: size.1,
                },
                wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth: 1,
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
    fn get_glyph(&mut self, desc: SpriteDescriptor) -> Option<Sprite> {
        match self.glyphs.entry(desc) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                // NOTE: we only need the allocation and coordinates now; the
                // rendering could be offloaded.
                let font = &self.fonts[desc.font()];
                let result = self.atlas_pipe.rasterize(font, desc);
                let sprite = if let Some((sprite, origin, size, data)) = result {
                    self.prepare.push((sprite.atlas, origin, size, data));
                    Some(sprite)
                } else {
                    None
                };
                entry.insert(sprite.clone());
                sprite
            }
        }
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
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Colour,
    ) {
        let time = std::time::Instant::now();
        let col = col.into();

        let for_glyph = |font: FontId, _, height: f32, glyph: Glyph| {
            let desc = SpriteDescriptor::new(font, glyph, height);
            if let Some(sprite) = pipe.get_glyph(desc) {
                let pos = pos + Vec2::from(glyph.position);
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
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Colour,
        effects: &[Effect<()>],
    ) -> Vec<Quad> {
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
        let col = col.into();
        let mut rects = vec![];

        let mut for_glyph = |font: FontId, _, height: f32, glyph: Glyph, _: usize, _: ()| {
            let desc = SpriteDescriptor::new(font, glyph, height);
            if let Some(sprite) = pipe.get_glyph(desc) {
                let pos = pos + Vec2::from(glyph.position);
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
                .map(|e| *e != Default::default())
                .unwrap_or(false)
        {
            let for_rect = |x1, x2, mut y, h: f32, _, _| {
                let y2 = y + h;
                if h < 1.0 {
                    // h too small can make the line invisible due to rounding
                    // In this case we prefer to push the line up (nearer text).
                    y = y2 - 1.0;
                }
                let quad = Quad::with_coords(pos + Vec2(x1, y), pos + Vec2(x2, y2));
                rects.push(quad);
            };
            text.glyphs_with_effects(effects, for_glyph, for_rect);
        } else {
            text.glyphs(|font, dpu, height, glyph| for_glyph(font, dpu, height, glyph, 0, ()));
        }

        self.duration += time.elapsed();
        rects
    }

    pub fn text_effects(
        &mut self,
        pipe: &mut Pipeline,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Colour>],
    ) -> Vec<(Quad, Colour)> {
        // Optimisation: use cheaper TextDisplay::glyphs method
        if effects.len() <= 1
            && effects
                .get(0)
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            let col = effects.get(0).map(|e| e.aux).unwrap_or(Colour::default());
            self.text(pipe, pass, pos, text, col);
            return vec![];
        }

        let time = std::time::Instant::now();
        let mut rects = vec![];

        let for_glyph = |font: FontId, _, height: f32, glyph: Glyph, _, col: Colour| {
            let desc = SpriteDescriptor::new(font, glyph, height);
            if let Some(sprite) = pipe.get_glyph(desc) {
                let pos = pos + Vec2::from(glyph.position);
                let a = pos + sprite.offset;
                let b = a + sprite.size;
                let (ta, tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
                let col = col.into();
                let instance = Instance { a, b, ta, tb, col };
                // TODO(opt): avoid calling repeatedly?
                self.atlas.rect(pass, sprite.atlas, instance);
            }
        };

        let for_rect = |x1, x2, mut y, h: f32, _, col: Colour| {
            let y2 = y + h;
            if h < 1.0 {
                // h too small can make the line invisible due to rounding
                // In this case we prefer to push the line up (nearer text).
                y = y2 - 1.0;
            }
            let quad = Quad::with_coords(pos + Vec2(x1, y), pos + Vec2(x2, y2));
            rects.push((quad, col));
        };

        text.glyphs_with_effects(effects, for_glyph, for_rect);

        self.duration += time.elapsed();
        rects
    }
}
