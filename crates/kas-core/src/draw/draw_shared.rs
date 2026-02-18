// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shared draw state

use super::color::Rgba;
use super::{DrawImpl, PassId};
use crate::ActionRedraw;
use crate::config::RasterConfig;
use crate::geom::{Quad, Size, Vec2};
use crate::text::{TextDisplay, format};
use std::any::Any;
use std::num::NonZeroU32;
use std::sync::Arc;
use thiserror::Error;

/// Identifier for an image allocation
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(NonZeroU32);

/// Handle for an image
///
/// Serves both to identify an allocated image and to track the number of users
/// via reference counting.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageHandle(ImageId, Arc<()>);

impl ImageHandle {
    /// Convert to an [`ImageId`]
    #[inline]
    pub fn id(&self) -> ImageId {
        self.0
    }
}

impl ImageId {
    /// Construct a new identifier from `u32` value not equal to 0
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    #[inline]
    pub const fn try_new(n: u32) -> Option<Self> {
        // We can't use ? or .map in a const fn so do it the tedious way:
        if let Some(nz) = NonZeroU32::new(n) {
            Some(ImageId(nz))
        } else {
            None
        }
    }
}

/// Image formats available for upload
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ImageFormat {
    /// 8-bit unsigned RGBA values (4 bytes per pixel)
    Rgba8,
}

/// Allocation failed: too large or zero sized
#[derive(Error, Debug)]
#[error("failed to allocate: size too large or zero-sized")]
pub struct AllocError;

/// Upload failed
#[derive(Error, Debug)]
pub enum UploadError {
    /// Image atlas not found
    #[error("image_upload: unknown atlas {0}")]
    AtlasIndex(u32),
    /// No allocation found for the [`ImageId`] used
    #[error("image_upload: allocation not found: {0:?}")]
    ImageId(ImageId),
    /// Image not within bounds of texture
    #[error("image_upload: texture coordinates not within bounds")]
    TextureCoordinates,
    /// Wrong data length
    #[error("image_upload: bad data length (received {0} bytes)")]
    DataLen(u32),
}

/// Shared draw state
///
/// A single [`SharedState`] instance is shared by all windows and draw contexts.
/// This struct is built over a [`DrawSharedImpl`] object provided by the graphics backend,
/// which may be accessed directly for a lower-level API (though most methods
/// are available through [`SharedState`] directly).
///
/// Note: all functionality is implemented through the [`DrawShared`] trait to
/// allow usage where the `DS` type parameter is unknown. Some functionality is
/// also implemented directly to avoid the need for downcasting.
pub struct SharedState<DS: DrawSharedImpl> {
    /// The graphics backend's [`DrawSharedImpl`] object
    pub draw: DS,
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl<DS: DrawSharedImpl> SharedState<DS> {
    /// Construct (this is only called by the graphics backend)
    pub fn new(draw: DS) -> Self {
        SharedState { draw }
    }
}

/// Interface over [`SharedState`]
///
/// All methods concern management of resources for drawing.
pub trait DrawShared {
    /// Allocate an image
    ///
    /// Use [`SharedState::image_upload`] to set contents of the new image.
    fn image_alloc(&mut self, format: ImageFormat, size: Size) -> Result<ImageHandle, AllocError>;

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    ///
    /// The `handle` must point to an existing allocation of size `(w, h)` and
    /// with image format `format` with `b` bytes-per-pixel such that
    /// `data.len() == b * w * h`. Data must be in row-major order.
    ///
    /// On success, this returns an [`ActionRedraw`] to indicate that any
    /// widgets using this image will require a redraw.
    fn image_upload(
        &mut self,
        handle: &ImageHandle,
        data: &[u8],
    ) -> Result<ActionRedraw, UploadError>;

    /// Potentially free an image
    ///
    /// The input `handle` is consumed. If this reduces its reference count to
    /// zero, then the image is freed.
    fn image_free(&mut self, handle: ImageHandle);

    /// Get the size of an image
    fn image_size(&self, handle: &ImageHandle) -> Option<Size>;
}

impl<DS: DrawSharedImpl> DrawShared for SharedState<DS> {
    #[inline]
    fn image_alloc(&mut self, format: ImageFormat, size: Size) -> Result<ImageHandle, AllocError> {
        self.draw
            .image_alloc(format, size)
            .map(|id| ImageHandle(id, Arc::new(())))
    }

    #[inline]
    fn image_upload(
        &mut self,
        handle: &ImageHandle,
        data: &[u8],
    ) -> Result<ActionRedraw, UploadError> {
        self.draw.image_upload(handle.0, data).map(|_| ActionRedraw)
    }

    #[inline]
    fn image_free(&mut self, handle: ImageHandle) {
        if let Ok(()) = Arc::try_unwrap(handle.1) {
            self.draw.image_free(handle.0);
        }
    }

    #[inline]
    fn image_size(&self, handle: &ImageHandle) -> Option<Size> {
        self.draw.image_size(handle.0)
    }
}

/// Implementation target for [`DrawShared`]
///
/// This is typically used via [`SharedState`].
pub trait DrawSharedImpl: Any {
    type Draw: DrawImpl;

    /// Get the maximum 2D texture size
    fn max_texture_dimension_2d(&self) -> u32;

    /// Set font raster config
    fn set_raster_config(&mut self, config: &RasterConfig);

    /// Allocate an image
    ///
    /// Use [`DrawSharedImpl::image_upload`] to set contents of the new image.
    fn image_alloc(&mut self, format: ImageFormat, size: Size) -> Result<ImageId, AllocError>;

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    fn image_upload(&mut self, id: ImageId, data: &[u8]) -> Result<(), UploadError>;

    /// Free an image allocation
    fn image_free(&mut self, id: ImageId);

    /// Query an image's size
    fn image_size(&self, id: ImageId) -> Option<Size>;

    /// Draw the image in the given `rect`
    fn draw_image(&self, draw: &mut Self::Draw, pass: PassId, id: ImageId, rect: Quad);

    /// Draw text with a list of color effects
    ///
    /// Text is drawn from `pos` and clipped to `bounding_box`.
    ///
    /// The `text` display must be prepared prior to calling this method.
    /// Typically this is done using a [`crate::theme::Text`] object.
    fn draw_text_effects(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        pos: Vec2,
        bounding_box: Quad,
        text: &TextDisplay,
        palette: &[Rgba],
        tokens: &[(u32, format::Colors)],
    );

    /// Draw text decorations (e.g. underlines)
    ///
    /// The `text` display must be prepared prior to calling this method.
    /// Typically this is done using a [`crate::theme::Text`] object.
    fn decorate_text(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        pos: Vec2,
        bounding_box: Quad,
        text: &TextDisplay,
        palette: &[Rgba],
        decorations: &[(u32, format::Decoration)],
    );
}
