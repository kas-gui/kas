// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Basic shapes

use super::{atlas, color_to_u32};
use kas::cast::{Cast, CastFloat, Conv};
use kas::draw::{AllocError, DrawImpl, DrawSharedImpl, PassId, PassType, WindowCommon};
use kas::draw::{ImageFormat, ImageId, color};
use kas::geom::{Quad, Size, Vec2};
use kas::prelude::{Offset, Rect};
use kas::runner::{self, RunError};
use kas::text::{self, raster};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use softbuffer::Buffer;

#[derive(Clone, Debug, Default)]
struct PassData {
    rects: Vec<(Quad, color::Rgba)>,
    lines: Vec<(Vec2, Vec2, color::Rgba)>,
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
        // TODO: draw correct width
        let _ = width;

        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }

        self.passes[pass].lines.push((p1, p2, col));
    }

    pub fn render(&mut self, pass: usize, buffer: &mut [u32], size: (usize, usize)) {
        let Some(pass) = self.passes.get_mut(pass) else {
            return;
        };

        for (rect, col) in pass.rects.drain(..) {
            let x0: usize = rect.a.0.cast_nearest();
            let x1: usize = rect.b.0.cast_nearest();
            let x1 = x1.min(size.0);

            let y0: usize = rect.a.1.cast_nearest();
            let y1: usize = rect.b.1.cast_nearest();
            let y1 = y1.min(size.1);

            let c = color_to_u32(col);

            for y in y0..y1 {
                let offset = y * size.0;
                buffer[offset + x0..offset + x1].fill(c);
            }
        }

        for (mut p1, mut p2, col) in pass.lines.drain(..) {
            let c = color_to_u32(col);
            if (p2.0 - p1.0).abs() >= (p2.1 - p1.1).abs() {
                if p2.0 < p1.0 {
                    std::mem::swap(&mut p1, &mut p2);
                }

                let x0: usize = p1.0.cast_nearest();
                let x1: usize = p2.0.cast_nearest();
                let x1 = x1.min(size.0);
                let xdi = 1.0 / (p2.0 - p1.0);
                let yd = p2.1 - p1.1;

                for x in x0..x1 {
                    let l = (f32::conv(x) - p1.0) * xdi;
                    let y: usize = (p1.1 + l * yd).cast_nearest();
                    buffer[y * size.0 + x] = c;
                }
            } else {
                if p2.1 < p1.1 {
                    std::mem::swap(&mut p1, &mut p2);
                }

                let y0: usize = p1.1.cast_nearest();
                let y1: usize = p2.1.cast_nearest();
                let y1 = y1.min(size.1);
                let ydi = 1.0 / (p2.1 - p1.1);
                let xd = p2.0 - p1.0;

                for y in y0..y1 {
                    let l = (f32::conv(y) - p1.1) * ydi;
                    let x: usize = (p1.0 + l * xd).cast_nearest();
                    buffer[y * size.0 + x] = c;
                }
            }
        }
    }
}
