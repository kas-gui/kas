// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS graphics backend over [softbuffer]
//!
//! This crate implements a KAS's drawing APIs over [softbuffer].
//!
//! This crate supports themes via the [`kas::theme`].
//!
//! [softbuffer]: https://github.com/rust-windowing/softbuffer

use super::color_to_u32;
use kas::cast::{Cast, CastFloat, Conv};
use kas::draw::{AllocError, DrawImpl, DrawSharedImpl, PassId, PassType, WindowCommon};
use kas::draw::{ImageFormat, ImageId, color};
use kas::geom::{Quad, Vec2};
use kas::prelude::{Offset, Rect};
use kas::runner::{self, RunError};
use kas::text;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use softbuffer::Buffer;

#[derive(Debug)]
struct ClipRegion {
    rect: Rect,
    offset: Offset,
    order: u64,
}

impl Default for ClipRegion {
    fn default() -> Self {
        ClipRegion {
            rect: Rect::ZERO,
            offset: Offset::ZERO,
            order: 1,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct PassData {
    rects: Vec<(Quad, color::Rgba)>,
    lines: Vec<(Vec2, Vec2, color::Rgba)>,
}

kas::impl_scope! {
    #[impl_default]
    pub struct Draw {
        pub(crate) common: WindowCommon,
        clip_regions: Vec<ClipRegion> = vec![Default::default()],
        passes: Vec<PassData>,
    }
}

impl DrawImpl for Draw {
    fn common_mut(&mut self) -> &mut WindowCommon {
        &mut self.common
    }

    fn new_pass(
        &mut self,
        parent_pass: PassId,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> PassId {
        let parent = match class {
            PassType::Clip => &self.clip_regions[parent_pass.pass()],
            PassType::Overlay => {
                // NOTE: parent_pass.pass() is always zero in this case since
                // this is only invoked from the Window (root).
                &self.clip_regions[0]
            }
        };
        let order = match class {
            PassType::Clip => (parent.order << 4) + 1,
            PassType::Overlay => (parent.order << 16) + 1,
        };
        let rect = rect - parent.offset;
        let offset = offset + parent.offset;
        let rect = rect.intersection(&parent.rect).unwrap_or(Rect::ZERO);
        let pass = self.clip_regions.len().cast();
        self.clip_regions.push(ClipRegion {
            rect,
            offset,
            order,
        });
        PassId::new(pass)
    }

    #[inline]
    fn get_clip_rect(&self, pass: PassId) -> Rect {
        let region = &self.clip_regions[pass.pass()];
        region.rect + region.offset
    }

    fn rect(&mut self, pass: PassId, rect: Quad, col: color::Rgba) {
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

    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: color::Rgba) {
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

    fn line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, _: f32, col: color::Rgba) {
        let pass = pass.pass();
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, Default::default());
        }

        self.passes[pass].lines.push((p1, p2, col));
    }
}

impl Draw {
    pub fn render(&mut self, buffer: &mut [u32], size: (usize, usize)) {
        // Order passes to ensure overlays are drawn after other content
        let mut passes: Vec<_> = self
            .clip_regions
            .iter()
            .map(|pass| pass.order)
            .enumerate()
            .collect();
        // Note that sorting is stable (does not re-order equal elements):
        passes.sort_by_key(|pass| pass.1);

        for (pass, _) in passes.drain(..) {
            if let Some(pass) = self.passes.get_mut(pass) {
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

        // Keep only first clip region (which is the entire window)
        self.clip_regions.truncate(1);
    }
}

#[derive(Default)]
pub struct Shared {
    text: kas::text::raster::State,
}

impl DrawSharedImpl for Shared {
    type Draw = Draw;

    fn max_texture_dimension_2d(&self) -> u32 {
        todo!()
    }

    fn set_raster_config(&mut self, config: &kas::config::RasterConfig) {
        self.text.set_raster_config(config);
    }

    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, AllocError> {
        todo!()
    }

    fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat) {
        todo!()
    }

    fn image_free(&mut self, id: ImageId) {
        todo!()
    }

    fn image_size(&self, id: ImageId) -> Option<(u32, u32)> {
        todo!()
    }

    fn draw_image(&self, draw: &mut Draw, pass: PassId, id: ImageId, rect: Quad) {
        todo!()
    }

    fn draw_text(
        &mut self,
        draw: &mut Draw,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &text::TextDisplay,
        col: color::Rgba,
    ) {
        todo!()
    }

    fn draw_text_effects(
        &mut self,
        draw: &mut Draw,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &text::TextDisplay,
        effects: &[text::Effect],
        colors: &[color::Rgba],
    ) {
        todo!()
    }
}
