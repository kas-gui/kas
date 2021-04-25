// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::{atlases, ShaderManager};
use ab_glyph::{Font, FontRef};
use kas::cast::*;
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Vec2};
use kas::text::fonts::{fonts, FontId};
use kas::text::{Glyph, GlyphId, TextDisplay};
use std::collections::hash_map::{Entry, HashMap};
use std::mem::size_of;

/// Scale multiplier for fixed-precision
///
/// This should be `1 << n` for `n` bits of sub-pixel precision.
const SCALE_MULT: f32 = 16.0;

/// A Sprite descriptor
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains those properties.
// TODO(opt): faster Hash
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct SpriteDescriptor {
    font: u16,
    glyph: u16,
    height: u32,
}

impl SpriteDescriptor {
    fn new(font: FontId, glyph: GlyphId, height: f32) -> Self {
        SpriteDescriptor {
            font: font.get().cast(),
            glyph: glyph.0,
            height: (height * SCALE_MULT).cast_nearest(),
        }
    }

    fn font(self) -> usize {
        self.font.cast()
    }

    fn height(self) -> f32 {
        f32::conv(self.height) / SCALE_MULT
    }
}

/// A Sprite
///
/// A "sprite" is a glyph rendered to a texture with fixed properties. This
/// struct contains everything needed to draw from the sprite.
#[derive(Clone, Debug)]
struct Sprite {
    atlas: u32,
    tex_quad: Quad,
}

impl atlases::Pipeline<Instance> {
    fn rasterize(
        &mut self,
        font: &FontRef<'static>,
        desc: SpriteDescriptor,
    ) -> Option<(Sprite, (u32, u32), (u32, u32), Vec<u8>)> {
        let glyph = ab_glyph::Glyph {
            id: ab_glyph::GlyphId(desc.glyph),
            scale: desc.height().into(),
            position: Default::default(),
        };
        let outline = font.outline_glyph(glyph)?;

        let bounds = outline.px_bounds();
        let size = bounds.max - bounds.min;
        let size = (u32::conv_trunc(size.x), u32::conv_trunc(size.y));
        println!("bounds: {:?}, size: {:?}", bounds, size);

        let (atlas, _, origin, tex_quad) = match self.allocate(size) {
            Ok(result) => result,
            Err(_) => {
                log::warn!("text_pipe: failed to allocate glyph with size {:?}", size);
                return None;
            }
        };

        let mut data = Vec::new();
        data.resize(usize::conv(size.0 * size.1), 0u8);
        outline.draw(|x, y, c| {
            // Convert to u8 with saturating conversion, rounding down:
            data[usize::conv((y * size.0) + x)] = (c * 256.0) as u8;
        });

        let sprite = Sprite { atlas, tex_quad };

        Some((sprite, origin, size, data))
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
                module: &shaders.vert_tex_quad,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Instance>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float2,
                        1 => Float2,
                        2 => Float2,
                        3 => Float2,
                    ],
                }],
            },
            &shaders.frag_image,
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
        println!("prepare_fonts: have {}", n2);
    }

    /// Write to textures
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.atlas_pipe.prepare(device);

        for (atlas, origin, size, data) in self.prepare.drain(..) {
            println!("origin={:?}, size={:?}, len={}", origin, size, data.len());
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
        bounds: Vec2,
        offset: Vec2,
        text: &TextDisplay,
        col: Colour,
    ) {
        let time = std::time::Instant::now();

        let _ = (bounds, col); // TODO
        let offset = pos + offset; // TODO: if we don't use bounds, we can just pass the sum

        let for_glyph = |font: FontId, _, height: f32, glyph: Glyph| {
            let desc = SpriteDescriptor::new(font, glyph.id, height);
            if let Some(sprite) = pipe.get_glyph(desc) {
                let pos = offset + Vec2::from(glyph.position);
                // FIXME:
                let offset = Vec2::ZERO;
                let size = Vec2(8.0, 12.0);
                let a = pos - offset;
                let b = a + size;
                let (ta, tb) = (sprite.tex_quad.a, sprite.tex_quad.b);
                let instance = Instance { a, b, ta, tb };
                // TODO(opt): avoid calling repeatedly?
                self.atlas.rect(pass, sprite.atlas, instance);
            }
        };
        text.glyphs(for_glyph);

        self.duration += time.elapsed();
    }
}
