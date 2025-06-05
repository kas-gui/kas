// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing pipeline

use super::images::{Images as Pipeline, InstanceA, Window};
use kas::cast::traits::*;
use kas::config::RasterConfig;
use kas::draw::{color::Rgba, AllocError, PassId};
use kas::geom::{Quad, Rect, Vec2};
use kas_text::fonts::{self, FaceId};
use kas_text::{Effect, Glyph, GlyphId, TextDisplay};
use rustc_hash::FxHashMap as HashMap;

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

/// A pipeline for rendering text
pub struct State {
    rasterer: Rasterer,
    #[allow(unused)]
    sb_align: bool,
    #[allow(unused)]
    hint: bool,
    config: Config,
    glyphs: HashMap<SpriteDescriptor, Sprite>,
    #[allow(clippy::type_complexity)]
    pub(super) prepare: Vec<(u32, (u32, u32), (u32, u32), Vec<u8>)>,
    scale_cx: swash::scale::ScaleContext,
}

impl State {
    pub fn new() -> Self {
        State {
            rasterer: Default::default(),
            sb_align: false,
            hint: false,
            config: Config::default(),
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
}

impl Pipeline {
    /// Raster a sequence of glyphs
    #[inline]
    fn raster_glyphs(
        &mut self,
        face_id: FaceId,
        dpem: f32,
        mut glyphs: impl Iterator<Item = Glyph>,
    ) {
        // NOTE: we only need the allocation and coordinates now; the
        // rendering could be offloaded (though this may not be useful).

        match self.text.rasterer {
            #[cfg(feature = "ab_glyph")]
            Rasterer::AbGlyph => self.raster_ab_glyph(face_id, dpem, &mut glyphs),
            Rasterer::Swash => self.raster_swash(face_id, dpem, &mut glyphs),
        }
    }

    #[cfg(feature = "ab_glyph")]
    fn raster_ab_glyph(
        &mut self,
        face_id: FaceId,
        dpem: f32,
        glyphs: &mut dyn Iterator<Item = Glyph>,
    ) {
        use ab_glyph::Font;

        let face_store = fonts::library().get_face_store(face_id);

        for glyph in glyphs {
            let desc = SpriteDescriptor::new(&self.text.config, face_id, glyph, dpem);
            if self.text.glyphs.contains_key(&desc) {
                continue;
            }

            let (mut x, y) = desc.fractional_position(&self.text.config);
            if self.text.sb_align
                && desc.dpem(&self.text.config) >= self.text.config.subpixel_threshold
            {
                let sf = face_store.face_ref().scale_by_dpem(dpem);
                x -= sf.h_side_bearing(glyph.id);
            }

            let font = face_store.ab_glyph();
            let scale = dpem * font.height_unscaled() / font.units_per_em().unwrap();
            let glyph = ab_glyph::Glyph {
                id: ab_glyph::GlyphId(glyph.id.0),
                scale: scale.into(),
                position: ab_glyph::point(x, y),
            };
            let Some(outline) = font.outline_glyph(glyph) else {
                log::warn!("raster_glyphs failed: unable to outline glyph");
                self.text.glyphs.insert(desc, Sprite::default());
                continue;
            };

            let bounds = outline.px_bounds();
            let offset: (i32, i32) = (bounds.min.x.cast_trunc(), bounds.min.y.cast_trunc());
            let size = bounds.max - bounds.min;
            let size = (u32::conv_trunc(size.x), u32::conv_trunc(size.y));
            if size.0 == 0 || size.1 == 0 {
                // Ignore this common error
                self.text.glyphs.insert(desc, Sprite::default());
                continue;
            }

            let mut data = vec![0; usize::conv(size.0 * size.1)];
            outline.draw(|x, y, c| {
                // Convert to u8 with saturating conversion, rounding down:
                data[usize::conv((y * size.0) + x)] = (c * 256.0) as u8;
            });

            let Ok((atlas, _, origin, tex_quad)) = self.atlas_a.allocate(size) else {
                log::warn!("raster_glyphs failed: unable to allocate");
                self.text.glyphs.insert(desc, Sprite::default());
                continue;
            };

            self.text.prepare.push((atlas, origin, size, data));

            self.text.glyphs.insert(desc, Sprite {
                atlas,
                valid: true,
                size: Vec2(size.0.cast(), size.1.cast()),
                offset: Vec2(offset.0.cast(), offset.1.cast()),
                tex_quad,
            });
        }
    }

    // NOTE: using dyn Iterator over impl Iterator is slightly slower but saves 2-4kB
    fn raster_swash(
        &mut self,
        face_id: FaceId,
        dpem: f32,
        glyphs: &mut dyn Iterator<Item = Glyph>,
    ) {
        use swash::scale::{image::Content, Render, Source, StrikeWith};
        use swash::zeno::{Angle, Format, Transform};

        let face = fonts::library().get_face_store(face_id);
        let font = face.swash();
        let synthesis = face.synthesis();

        let mut scaler = self
            .text
            .scale_cx
            .builder(font)
            .size(dpem)
            .hint(self.text.hint)
            .variations(
                synthesis
                    .variation_settings()
                    .into_iter()
                    .map(|(tag, value)| (swash::tag_from_bytes(&tag.to_be_bytes()), *value)),
            )
            .build();

        let sources = &[
            // TODO: Support coloured rendering? These can replace Source::Bitmap
            // Source::ColorOutline(0),
            // Source::ColorBitmap(StrikeWith::BestFit),
            Source::Bitmap(StrikeWith::BestFit),
            Source::Outline,
        ];

        // Faux italic skew:
        let transform = synthesis
            .skew()
            .map(|angle| Transform::skew(Angle::from_degrees(angle), Angle::ZERO));
        // Faux bold:
        let embolden = if synthesis.embolden() { dpem * 0.02 } else { 0.0 };

        for glyph in glyphs {
            let desc = SpriteDescriptor::new(&self.text.config, face_id, glyph, dpem);
            if self.text.glyphs.contains_key(&desc) {
                continue;
            }

            let Some(image) = Render::new(sources)
                .format(Format::Alpha)
                .offset(desc.fractional_position(&self.text.config).into())
                .transform(transform)
                .embolden(embolden)
                .render(&mut scaler, desc.glyph().0.into())
            else {
                log::warn!("raster_glyphs failed: unable to construct renderer");
                self.text.glyphs.insert(desc, Sprite::default());
                continue;
            };

            let offset = (image.placement.left, -image.placement.top);
            let size = (image.placement.width, image.placement.height);
            if size.0 == 0 || size.1 == 0 {
                // Ignore this common error
                self.text.glyphs.insert(desc, Sprite::default());
                continue;
            }

            let sprite = match image.content {
                Content::Mask => {
                    let Ok((atlas, _, origin, tex_quad)) = self.atlas_a.allocate(size) else {
                        log::warn!("raster_glyphs failed: unable to allocate");
                        self.text.glyphs.insert(desc, Sprite::default());
                        continue;
                    };

                    self.text.prepare.push((atlas, origin, size, image.data));

                    Sprite {
                        atlas,
                        valid: true,
                        size: Vec2(size.0.cast(), size.1.cast()),
                        offset: Vec2(offset.0.cast(), offset.1.cast()),
                        tex_quad,
                    }
                }
                Content::SubpixelMask => unimplemented!(),
                Content::Color => unimplemented!(),
            };

            self.text.glyphs.insert(desc, sprite);
        }
    }
}

impl Window {
    pub fn text(
        &mut self,
        pipe: &mut Pipeline,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
    ) {
        let rect = Quad::conv(rect);

        for run in text.runs() {
            let face = run.face_id();
            let dpem = run.dpem();
            for glyph in run.glyphs() {
                let desc = SpriteDescriptor::new(&pipe.text.config, face, glyph, dpem);
                let sprite = match pipe.text.glyphs.get(&desc) {
                    Some(sprite) => sprite,
                    None => {
                        pipe.raster_glyphs(face, dpem, run.glyphs());
                        match pipe.text.glyphs.get(&desc) {
                            Some(sprite) => sprite,
                            None => continue,
                        }
                    }
                };
                if sprite.is_valid() {
                    let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                    let b = a + sprite.size;
                    if let Some(instance) = InstanceA::new(rect, a, b, sprite.tex_quad, col) {
                        self.atlas_a.rect(pass, sprite.atlas, instance);
                    }
                }
            }
        }
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
        // Optimisation: use cheaper TextDisplay::runs method
        if effects.len() <= 1
            && effects
                .first()
                .map(|e| e.flags == Default::default())
                .unwrap_or(true)
        {
            self.text(pipe, pass, rect, text, col);
            return;
        }

        let rect = Quad::conv(rect);

        for run in text.runs_with_effects(effects, ()) {
            let face = run.face_id();
            let dpem = run.dpem();
            let for_glyph = |glyph: Glyph, _: usize, _: ()| {
                let desc = SpriteDescriptor::new(&pipe.text.config, face, glyph, dpem);
                let sprite = match pipe.text.glyphs.get(&desc) {
                    Some(sprite) => sprite,
                    None => {
                        pipe.raster_glyphs(face, dpem, run.glyphs());
                        match pipe.text.glyphs.get(&desc) {
                            Some(sprite) => sprite,
                            None => return,
                        }
                    }
                };
                if sprite.is_valid() {
                    let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                    let b = a + sprite.size;
                    if let Some(instance) = InstanceA::new(rect, a, b, sprite.tex_quad, col) {
                        self.atlas_a.rect(pass, sprite.atlas, instance);
                    }
                }
            };

            let for_rect = |x1, x2, y: f32, h: f32, _, _| {
                let y = y.ceil();
                let y2 = y + h.ceil();
                if let Some(quad) = Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2))
                    .intersection(&rect)
                {
                    draw_quad(quad);
                }
            };

            run.glyphs_with_effects(for_glyph, for_rect);
        }
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
        // Optimisation: use cheaper TextDisplay::runs method
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

        for run in text.runs_with_effects(effects, Rgba::BLACK) {
            let face = run.face_id();
            let dpem = run.dpem();
            let for_glyph = |glyph: Glyph, _, col: Rgba| {
                let desc = SpriteDescriptor::new(&pipe.text.config, face, glyph, dpem);
                let sprite = match pipe.text.glyphs.get(&desc) {
                    Some(sprite) => sprite,
                    None => {
                        pipe.raster_glyphs(face, dpem, run.glyphs());
                        match pipe.text.glyphs.get(&desc) {
                            Some(sprite) => sprite,
                            None => return,
                        }
                    }
                };
                if sprite.is_valid() {
                    let a = rect.a + Vec2::from(glyph.position).floor() + sprite.offset;
                    let b = a + sprite.size;
                    if let Some(instance) = InstanceA::new(rect, a, b, sprite.tex_quad, col) {
                        self.atlas_a.rect(pass, sprite.atlas, instance);
                    }
                }
            };

            let for_rect = |x1, x2, y: f32, h: f32, _, col: Rgba| {
                let y = y.ceil();
                let y2 = y + h.ceil();
                if let Some(quad) = Quad::from_coords(rect.a + Vec2(x1, y), rect.a + Vec2(x2, y2))
                    .intersection(&rect)
                {
                    draw_quad(quad, col);
                }
            };

            run.glyphs_with_effects(for_glyph, for_rect);
        }
    }
}
