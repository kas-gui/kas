// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Basic shapes

use super::color_to_u32;
use kas::cast::traits::*;
use kas::draw::PassId;
use kas::draw::color;
use kas::geom::{Coord, Quad, Vec2};
use kas::prelude::{Offset, Rect};

#[derive(Clone, Debug, Default)]
struct PassData {
    rects: Vec<(Quad, color::Rgba)>,
    lines: Vec<(Vec2, Vec2, f32, color::Rgba)>,
}

#[derive(Debug, Default)]
pub struct Draw {
    passes: Vec<PassData>,
}

impl Draw {
    pub fn rect(&mut self, pass: PassId, rect: Quad, col: color::Rgba) {
        if !(rect.a < rect.b) {
            // zero / negative size: nothing to draw
            return;
        }

        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }

        self.passes[pass].rects.push((rect, col));
    }

    pub fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: color::Rgba) {
        let aa = outer.a;
        let bb = outer.b;
        let mut cc = inner.a;
        let mut dd = inner.b;

        if !(aa < bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !(aa <= cc) || !(cc <= bb) {
            cc = aa;
        }
        if !(aa <= dd) || !(dd <= bb) {
            dd = bb;
        }
        if !(cc <= dd) {
            dd = cc;
        }

        let ac = Vec2(aa.0, cc.1);
        let ad = Vec2(aa.0, dd.1);
        let bc = Vec2(bb.0, cc.1);
        let bd = Vec2(bb.0, dd.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        self.rect(pass, Quad::from_coords(aa, bc), col);
        self.rect(pass, Quad::from_coords(ad, bb), col);
        self.rect(pass, Quad::from_coords(ac, cd), col);
        self.rect(pass, Quad::from_coords(dc, bd), col);
    }

    pub fn line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, width: f32, col: color::Rgba) {
        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }

        let r = 0.5 * width;
        self.passes[pass].lines.push((p1, p2, r, col));
    }

    pub fn render(
        &mut self,
        pass: usize,
        buffer: &mut [u32],
        size: (usize, usize),
        clip_rect: Rect,
        offset: Offset,
    ) {
        let Some(pass) = self.passes.get_mut(pass) else {
            return;
        };
        let (clip_p, clip_q) = (clip_rect.pos, clip_rect.pos2());

        for (rect, col) in pass.rects.drain(..) {
            let p = (Coord::conv_nearest(rect.a) - offset).clamp(clip_p, clip_q);
            let q = (Coord::conv_nearest(rect.b) - offset).clamp(clip_p, clip_q);
            let (x0, x1): (usize, usize) = (p.0.cast(), q.0.cast());
            let c = color_to_u32(col);

            for y in p.1..q.1 {
                let offset = usize::conv(y) * size.0;
                buffer[offset + x0..offset + x1].fill(c);
            }
        }

        for (p1, p2, r, col) in pass.lines.drain(..) {
            let c = color_to_u32(col);
            let (dx, dy) = (p2.0 - p1.0, p2.1 - p1.1);
            let d_inv = 1.0 / (dx * dx + dy * dy).sqrt();

            // Target rect within which the line may be drawn
            // NOTE: this is inefficient, but we don't draw many lines anyway
            let (mut a, mut b) = (p1, p2);
            if a.0 > b.0 {
                std::mem::swap(&mut a.0, &mut b.0);
            }
            if a.1 > b.1 {
                std::mem::swap(&mut a.1, &mut b.1);
            }
            a -= Vec2::splat(r);
            b += Vec2::splat(r);
            let a = (Coord::conv_nearest(a) - offset).clamp(clip_p, clip_q);
            let b = (Coord::conv_nearest(b) - offset).clamp(clip_p, clip_q);

            for y in a.1..b.1 {
                let d1 = dx * (f32::conv(y + offset.1) - p1.1);

                for x in a.0..b.0 {
                    let d2 = dy * (f32::conv(x + offset.0) - p1.0);
                    let dist = (d1 - d2).abs() * d_inv;
                    if dist <= r {
                        buffer[usize::conv(y) * size.0 + usize::conv(x)] = c;
                    }
                }
            }
        }
    }
}
