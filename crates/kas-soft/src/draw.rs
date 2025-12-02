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

use kas::cast::Cast;
use kas::draw::{AllocError, DrawImpl, DrawSharedImpl, PassId, PassType, WindowCommon};
use kas::draw::{ImageFormat, ImageId, color};
use kas::geom::{Quad, Vec2};
use kas::prelude::{Offset, Rect};
use kas::runner::{self, RunError};
use kas::text;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

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

kas::impl_scope! {
    #[impl_default]
    pub struct Draw {
        pub(crate) common: WindowCommon,
        clip_regions: Vec<ClipRegion> = vec![Default::default()],
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
        todo!()
    }

    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: color::Rgba) {
        todo!()
    }
}

pub struct Shared {}

impl DrawSharedImpl for Shared {
    type Draw = Draw;

    fn max_texture_dimension_2d(&self) -> u32 {
        todo!()
    }

    fn set_raster_config(&mut self, config: &kas::config::RasterConfig) {
        todo!()
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
