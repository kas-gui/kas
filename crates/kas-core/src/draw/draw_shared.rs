// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shared draw state

use super::color::Rgba;
use super::{DrawImpl, PassId};
use crate::cast::Cast;
use crate::geom::{Quad, Rect, Size};
use crate::text::{Effect, TextDisplay};
use crate::theme::RasterConfig;
use std::any::Any;
use std::num::NonZeroU32;
use std::rc::Rc;
use thiserror::Error;

/// Identifier for an image allocation
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(NonZeroU32);

/// Handle for an image
///
/// Serves both to identify an allocated image and to track the number of users
/// via reference counting.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageHandle(ImageId, Rc<()>);

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
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
}

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<DS: DrawSharedImpl> SharedState<DS> {
    /// Construct (this is only called by the shell)
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
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageHandle, AllocError>;

    /// Upload an image to the GPU
    ///
    /// This should be called at least once on each image before display. May be
    /// called again to update the image contents.
    ///
    /// `handle` must refer to an allocation of some size `(w, h)`, such that
    /// `data.len() == b * w * h` where `b` is the number of bytes per pixel,
    /// according to `format`. Data must be in row-major order.
    fn image_upload(&mut self, handle: &ImageHandle, data: &[u8], format: ImageFormat);

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
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageHandle, AllocError> {
        self.draw
            .image_alloc(size)
            .map(|id| ImageHandle(id, Rc::new(())))
    }

    #[inline]
    fn image_upload(&mut self, handle: &ImageHandle, data: &[u8], format: ImageFormat) {
        self.draw.image_upload(handle.0, data, format);
    }

    #[inline]
    fn image_free(&mut self, handle: ImageHandle) {
        if let Ok(()) = Rc::try_unwrap(handle.1) {
            self.draw.image_free(handle.0);
        }
    }

    #[inline]
    fn image_size(&self, handle: &ImageHandle) -> Option<Size> {
        self.draw.image_size(handle.0).map(|size| size.cast())
    }
}

/// Trait over shared data of draw object
///
/// This is typically used via [`SharedState`].
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawSharedImpl: Any {
    type Draw: DrawImpl;

    /// Get the maximum 2D texture size
    fn max_texture_dimension_2d(&self) -> u32;

    /// Set font raster config
    fn set_raster_config(&mut self, config: &RasterConfig);

    /// Allocate an image
    ///
    /// Use [`DrawSharedImpl::image_upload`] to set contents of the new image.
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, AllocError>;

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
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
    );

    /// Draw text with a colour and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    fn draw_text_effects(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    );

    /// Draw text with effects
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    fn draw_text_effects_rgba(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    );
}
