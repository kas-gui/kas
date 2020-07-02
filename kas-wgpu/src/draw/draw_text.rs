// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use wgpu_glyph::ab_glyph;
use wgpu_glyph::Extra;

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, DrawText, Pass};
use kas::geom::Vec2;
use kas::text::PreparedText;

fn to_point(Vec2(x, y): Vec2) -> ab_glyph::Point {
    ab_glyph::Point { x, y }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text(&mut self, pass: Pass, pos: Vec2, col: Colour, text: &PreparedText) {
        // TODO: perhaps glyph_brush can accept an offset for all glyphs?
        let glyphs = text.positioned_glyphs(pos);
        let extra = (0..text.num_parts())
            .map(|_| Extra {
                color: col.into(),
                z: pass.depth(),
            })
            .collect();
        let min = to_point(pos);
        let max = to_point(pos + text.bounds());
        let bounds = ab_glyph::Rect { min, max };
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds);
    }
}
