// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shared draw state

use super::color::Rgba;
use super::{images, DrawImpl, ImageError, ImageFormat, ImageId, PassId};
use crate::geom::{Quad, Size, Vec2};
use crate::text::{Effect, TextDisplay};
use std::any::Any;
use std::path::Path;

/// Shared draw state
///
/// A single [`SharedState`] instance is shared by all windows and draw contexts.
/// This struct is built over a [`DrawSharedImpl`] object provided by the shell,
/// which may be accessed directly for a lower-level API (though most methods
/// are available through [`SharedState`] directly).
///
/// Note: all functionality is implemented through the [`DrawShared`] trait to
/// allow usage where the `DS` type parameter is unknown. Some functionality is
/// also implemented directly to avoid the need for downcasting.
pub struct SharedState<DS: DrawSharedImpl> {
    /// The shell's [`DrawSharedImpl`] object
    pub draw: DS,
    images: images::Images,
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<DS: DrawSharedImpl> SharedState<DS> {
    /// Construct (this is only called by the shell)
    pub fn new(draw: DS) -> Self {
        let images = images::Images::new();
        SharedState { draw, images }
    }
}

/// Interface over [`SharedState`]
///
/// All methods concern management of resources for drawing.
pub trait DrawShared {
    /// Allocate an image
    ///
    /// Use [`SharedState::image_upload`] to set contents of the new image.
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, ImageError>;

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    ///
    /// `id` must refer to an allocation of some size `(w, h)`, such that
    /// `data.len() == b * w * h` where `b` is the number of bytes per pixel,
    /// according to `format`. Data must be in row-major order.
    fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat);

    /// Load an image from a path, autodetecting file type
    ///
    /// This deduplicates multiple loads of the same path, instead incrementing
    /// a reference count.
    fn image_from_path(&mut self, path: &Path) -> Result<ImageId, ImageError>;

    /// Remove a loaded image, by path
    ///
    /// This reduces the reference count and frees if zero.
    fn image_free_from_path(&mut self, path: &Path);

    /// Free an image
    fn image_free(&mut self, id: ImageId);

    /// Get the size of an image
    fn image_size(&self, id: ImageId) -> Option<Size>;
}

impl<DS: DrawSharedImpl> DrawShared for SharedState<DS> {
    #[inline]
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, ImageError> {
        self.draw.image_alloc(size)
    }

    #[inline]
    fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat) {
        self.draw.image_upload(id, data, format);
    }

    #[inline]
    fn image_from_path(&mut self, path: &Path) -> Result<ImageId, ImageError> {
        self.images.load_path(&mut self.draw, path)
    }

    #[inline]
    fn image_free_from_path(&mut self, path: &Path) {
        self.images.remove_path(&mut self.draw, path);
    }

    #[inline]
    fn image_free(&mut self, id: ImageId) {
        self.images.remove_id(&mut self.draw, id);
    }

    #[inline]
    fn image_size(&self, id: ImageId) -> Option<Size> {
        self.draw.image_size(id).map(|size| size.into())
    }
}

/// Trait over shared data of draw object
///
/// This is typically used via [`SharedState`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawSharedImpl: Any {
    type Draw: DrawImpl;

    /// Allocate an image
    ///
    /// Use [`DrawSharedImpl::image_upload`] to set contents of the new image.
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
    fn draw_image(&self, draw: &mut Self::Draw, pass: PassId, id: ImageId, rect: Quad);

    /// Draw text with a colour
    fn draw_text(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
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
        draw: &mut Self::Draw,
        pass: PassId,
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
        draw: &mut Self::Draw,
        pass: PassId,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    );
}
