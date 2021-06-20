// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shared draw state

use super::color::Rgba;
use super::{images, Draw, ImageError, ImageFormat, ImageId, Pass};
use crate::geom::{Quad, Size, Vec2};
use crate::text::{Effect, TextDisplay};
use std::path::Path;

/// Interface over a shared draw object
pub struct DrawShared<DS: DrawableShared> {
    pub draw: DS,
    images: images::Images,
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<DS: DrawableShared> DrawShared<DS> {
    /// Construct
    pub fn new(draw: DS) -> Self {
        let images = images::Images::new();
        DrawShared { draw, images }
    }
}

impl<DS: DrawableShared> DrawShared<DS> {
    /// Allocate an image
    ///
    /// Use [`DrawShared::upload`] to set contents of the new image.
    pub fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, ImageError> {
        self.draw.image_alloc(size)
    }

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    pub fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat) {
        self.draw.image_upload(id, data, format);
    }

    /// Load an image from a path, autodetecting file type
    ///
    /// This deduplicates multiple loads of the same path, instead incrementing
    /// a reference count.
    pub fn image_from_path(&mut self, path: &Path) -> Result<ImageId, ImageError> {
        self.images.load_path(&mut self.draw, path)
    }

    /// Remove a loaded image, by path
    ///
    /// This reduces the reference count and frees if zero.
    pub fn remove_image_from_path(&mut self, path: &Path) {
        self.images.remove_path(&mut self.draw, path);
    }

    /// Free an image
    pub fn remove_image(&mut self, id: ImageId) {
        self.images.remove_id(&mut self.draw, id);
    }

    /// Get the size of an image
    pub fn image_size(&self, id: ImageId) -> Option<Size> {
        self.draw.image_size(id).map(|size| size.into())
    }

    /// Draw the image in the given `rect`
    pub fn draw_image(&self, window: &mut DS::Draw, pass: Pass, id: ImageId, rect: Quad) {
        self.draw.draw_image(window, pass, id, rect)
    }

    /// Draw text with a colour
    pub fn draw_text(
        &mut self,
        window: &mut DS::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
    ) {
        self.draw.draw_text(window, pass, pos, text, col)
    }

    /// Draw text with a colour and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    pub fn draw_text_col_effects(
        &mut self,
        window: &mut DS::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) {
        self.draw
            .draw_text_col_effects(window, pass, pos, text, col, effects)
    }

    /// Draw text with effects
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    pub fn draw_text_effects(
        &mut self,
        window: &mut DS::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    ) {
        self.draw
            .draw_text_effects(window, pass, pos, text, effects)
    }
}

/// Trait over shared data of draw object
///
/// This is typically used via [`DrawShared`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait DrawableShared: 'static {
    type Draw: Draw;

    /// Allocate an image
    ///
    /// Use [`DrawableShared::upload`] to set contents of the new image.
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, ImageError>;

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat);

    /// Free an image allocation
    fn image_free(&mut self, id: ImageId);

    /// Query an image's size
    fn image_size(&self, id: ImageId) -> Option<(u32, u32)>;

    /// Draw the image in the given `rect`
    fn draw_image(&self, window: &mut Self::Draw, pass: Pass, id: ImageId, rect: Quad);

    /// Draw text with a colour
    fn draw_text(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
    );

    /// Draw text with a colour and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    fn draw_text_col_effects(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    );

    /// Draw text with effects
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    fn draw_text_effects(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    );
}
