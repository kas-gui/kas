// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Root drawing implementations

use super::{atlas, basic};
use kas::cast::Cast;
use kas::draw::{
    AllocError, DrawImpl, DrawSharedImpl, PassId, PassType, UploadError, WindowCommon,
};
use kas::draw::{ImageFormat, ImageId, color};
use kas::geom::{Quad, Size, Vec2};
use kas::prelude::{Offset, Rect};
use kas::text::{self};

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
        images: atlas::Window,
        clip_regions: Vec<ClipRegion> = vec![Default::default()],
        basic: basic::Draw,
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
        self.basic.rect(pass, rect, col);
    }

    #[inline]
    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: color::Rgba) {
        self.basic.frame(pass, outer, inner, col);
    }

    #[inline]
    fn line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, width: f32, col: color::Rgba) {
        self.basic.line(pass, p1, p2, width, col);
    }
}

impl Draw {
    pub fn resize(&mut self, size: Size) {
        self.clip_regions[0].rect.size = size;
    }

    pub fn render(&mut self, shared: &mut Shared, buffer: &mut [u32], size: (usize, usize)) {
        shared.images.prepare(&mut shared.text);

        // Order passes to ensure overlays are drawn after other content
        let mut passes: Vec<_> = self.clip_regions.iter().enumerate().collect();
        // Note that sorting is stable (does not re-order equal elements):
        passes.sort_by_key(|pass| pass.1.order);

        for (pass, clip) in passes.drain(..) {
            self.basic
                .render(pass, buffer, size, clip.rect, clip.offset);
            self.images
                .render(&shared.images, pass, buffer, size, clip.rect, clip.offset);
        }

        // Keep only first clip region (which is the entire window)
        self.clip_regions.truncate(1);
    }
}

pub struct Shared {
    images: atlas::Shared,
    text: kas::text::raster::State,
}

impl Default for Shared {
    fn default() -> Self {
        Shared {
            images: atlas::Shared::new(),
            text: Default::default(),
        }
    }
}

impl DrawSharedImpl for Shared {
    type Draw = Draw;

    #[inline]
    fn max_texture_dimension_2d(&self) -> u32 {
        atlas::MAX_TEX_SIZE.cast()
    }

    #[inline]
    fn set_raster_config(&mut self, config: &kas::config::RasterConfig) {
        self.images.set_raster_config(config);
        self.text.set_raster_config(config);
    }

    #[inline]
    fn image_alloc(&mut self, size: Size) -> Result<ImageId, AllocError> {
        self.images.alloc(size)
    }

    #[inline]
    fn image_upload(
        &mut self,
        id: ImageId,
        data: &[u8],
        format: ImageFormat,
    ) -> Result<(), UploadError> {
        self.images.upload(id, data, format)
    }

    #[inline]
    fn image_free(&mut self, id: ImageId) {
        self.images.free(id);
    }

    #[inline]
    fn image_size(&self, id: ImageId) -> Option<Size> {
        self.images.image_size(id)
    }

    fn draw_image(&self, draw: &mut Draw, pass: PassId, id: ImageId, rect: Quad) {
        if let Some((atlas, tex)) = self.images.get_im_atlas_coords(id) {
            draw.images.rect(pass, atlas, tex, rect);
        }
    }

    #[inline]
    fn draw_text(
        &mut self,
        draw: &mut Draw,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &text::TextDisplay,
        col: color::Rgba,
    ) {
        let time = std::time::Instant::now();
        self.text
            .text(&mut self.images, &mut draw.images, pass, pos, bb, text, col);
        draw.common.report_dur_text(time.elapsed());
    }

    fn draw_text_effects(
        &mut self,
        draw: &mut Draw,
        pass: PassId,
        pos: Vec2,
        bb: Quad,
        text: &text::TextDisplay,
        colors: &[color::Rgba],
        effects: &[text::Effect],
    ) {
        let time = std::time::Instant::now();
        self.text.text_effects(
            &mut self.images,
            &mut draw.images,
            pass,
            pos,
            bb,
            text,
            colors,
            effects,
            |quad, col| {
                draw.basic.rect(pass, quad, col);
            },
        );
        draw.common.report_dur_text(time.elapsed());
    }
}
