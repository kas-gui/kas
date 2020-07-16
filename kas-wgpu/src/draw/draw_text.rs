// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use wgpu_glyph::{ab_glyph, Extra, FontId, SectionGlyph};

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, DrawText, Pass};
use kas::geom::Vec2;
use kas::text::PreparedText;

fn to_point(Vec2(x, y): Vec2) -> ab_glyph::Point {
    ab_glyph::Point { x, y }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text(&mut self, pass: Pass, pos: Vec2, col: Colour, text: &PreparedText) {
        let pos = to_point(pos);
        let glyphs = text.positioned_glyphs(|_, font_id, scale, glyph| SectionGlyph {
            // Index fields are not used when drawing
            section_index: 0,
            byte_index: 0,
            glyph: ab_glyph::Glyph {
                id: glyph.id,
                scale,
                position: pos + glyph.position.into(),
            },
            font_id: FontId(font_id.get()),
        });
        let extra = vec![Extra {
            color: col.into(),
            z: pass.depth(),
        }];
        let min = pos;
        let max = pos + text.env().bounds.into();
        let bounds = ab_glyph::Rect { min, max };
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds);
    }
}
