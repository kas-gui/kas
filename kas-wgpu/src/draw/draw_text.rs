// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use wgpu_glyph::{ab_glyph, Extra, FontId, SectionGlyph};

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, DrawText, Pass, TextEffect};
use kas::geom::Vec2;
use kas::text::{fonts::fonts, Text};

fn to_point(Vec2(x, y): Vec2) -> ab_glyph::Point {
    ab_glyph::Point { x, y }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text_with_effects(
        &mut self,
        pass: Pass,
        pos: Vec2,
        offset: Vec2,
        text: &Text,
        effects: &[TextEffect],
    ) {
        // Check all fonts are loaded. TODO: move (only need to do this once per frame, at most).
        let fonts = fonts();
        let n1 = self.glyph_brush.fonts().len();
        let n2 = fonts.num_fonts();
        let mut count = n1;
        if n2 > n1 {
            for font in fonts.ab_glyph_fonts_from(n1) {
                let id = self.glyph_brush.add_font(font);
                assert_eq!(id.0, count);
                count += 1;
            }
        }

        let pos = to_point(pos);
        let offset = pos - to_point(offset);

        let mut section = 0;
        let mut next = 0;
        let mut next_start = effects.get(next).map(|e| e.start).unwrap_or(u32::MAX);
        let glyphs = text.positioned_glyphs(|_, font_id, scale, glyph| {
            while glyph.index >= next_start {
                section = next;
                next += 1;
                next_start = effects.get(next).map(|e| e.start).unwrap_or(u32::MAX);
            }
            SectionGlyph {
                section_index: section,
                byte_index: 0, // not used
                glyph: ab_glyph::Glyph {
                    id: glyph.id,
                    scale,
                    position: offset + glyph.position.into(),
                },
                font_id: FontId(font_id.get()),
            }
        });

        let mut col = Colour::grey(0.0);
        let mut col2 = col;
        let mut underline = false;
        let extra = effects
            .iter()
            .map(|effect| {
                if let Some(c) = effect.col {
                    if !underline {
                        col = c;
                    }
                    col2 = c;
                }
                if let Some(u) = effect.underline {
                    // HACK: draw a real underline (instead we just change colour)
                    underline = u;
                    col = if u { Colour::new(0.9, 0.2, 0.5) } else { col2 };
                }
                Extra {
                    color: col.into(),
                    z: pass.depth(),
                }
            })
            .collect();

        let min = pos;
        let max = pos + text.env().bounds.into();
        let bounds = ab_glyph::Rect { min, max };

        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds);
    }
}
