// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use wgpu_glyph::{ab_glyph, Extra, SectionGlyph};

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, Draw, DrawText, Pass, TextEffect};
use kas::geom::{Quad, Vec2};
use kas::text::fonts::{fonts, FontId};
use kas::text::{Effect, EffectFlags, Glyph, Text};

fn to_point(Vec2(x, y): Vec2) -> ab_glyph::Point {
    ab_glyph::Point { x, y }
}

fn ktv_to_point(kas::text::Vec2(x, y): kas::text::Vec2) -> ab_glyph::Point {
    ab_glyph::Point { x, y }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn prepare_fonts(&mut self) {
        let fonts = fonts();
        let n1 = self.glyph_brush.fonts().len();
        let n2 = fonts.num_fonts();
        if n2 > n1 {
            // TODO: use extra caching so we don't load font for each window
            let font_data = kas::text::fonts::fonts().font_data();
            for i in n1..n2 {
                let (data, index) = font_data.get_data(i);
                let font = ab_glyph::FontRef::try_from_slice_and_index(data, index).unwrap();
                let id = self.glyph_brush.add_font(font);
                assert_eq!(id.0, i);
            }
        }
    }

    fn text_with_effects(
        &mut self,
        pass: Pass,
        pos: Vec2,
        offset: Vec2,
        text: &Text,
        effects: &[TextEffect],
    ) {
        let time = std::time::Instant::now();
        let ab_pos = to_point(pos);
        let ab_offset = ab_pos - to_point(offset);

        let mut col = Colour::grey(0.0);
        let mut underline = false;
        let mut strikethrough = false;
        let mut v = Vec::with_capacity(effects.len());
        let mut extra = Vec::with_capacity(effects.len());
        for (i, e) in effects.iter().enumerate() {
            if let Some(c) = e.col {
                col = c;
            }
            if let Some(u) = e.underline {
                underline = u;
            }
            if let Some(s) = e.strikethrough {
                strikethrough = s;
            }
            v.push(Effect {
                start: e.start,
                flags: match (underline, strikethrough) {
                    (false, false) => EffectFlags::default(),
                    (true, false) => EffectFlags::UNDERLINE,
                    (false, true) => EffectFlags::STRIKETHROUGH,
                    (true, true) => EffectFlags::UNDERLINE | EffectFlags::STRIKETHROUGH,
                },
                aux: i,
            });
            extra.push(Extra {
                color: col.into(),
                z: pass.depth(),
            });
        }

        let mut glyphs = Vec::with_capacity(text.num_glyphs());
        let for_glyph = |font_id: FontId, _, height: f32, glyph: Glyph, aux| {
            glyphs.push(SectionGlyph {
                section_index: aux,
                byte_index: 0, // not used
                glyph: ab_glyph::Glyph {
                    id: ab_glyph::GlyphId(glyph.id.0),
                    scale: height.into(),
                    position: ab_offset + ktv_to_point(glyph.position),
                },
                font_id: wgpu_glyph::FontId(font_id.get()),
            });
        };
        let for_rect = |x1, x2, mut y, h: f32, aux: usize| {
            let y2 = y + h;
            if h < 1.0 {
                // h too small can make the line invisible due to rounding
                // In this case we prefer to push the line up (nearer text).
                y = y2 - 1.0;
            }
            let quad = Quad::with_coords(pos + Vec2(x1, y), pos + Vec2(x2, y2));
            self.rect(pass, quad, extra[aux].color.into());
        };
        text.glyphs_with_effects(&v, for_glyph, for_rect);

        let min = ab_pos;
        let max = ab_pos + ktv_to_point(text.env().bounds);
        let bounds = ab_glyph::Rect { min, max };

        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds);
        self.dur_text += time.elapsed();
    }
}
