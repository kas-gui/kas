// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use super::Scaling;
use kas::draw::{DrawShared, ImageHandle};
use kas::layout::LogicalSize;
use kas::prelude::*;
use kas::theme::MarginStyle;

/// Image loading errors
#[cfg(feature = "image")]
#[derive(thiserror::Error, Debug)]
pub enum ImageError {
    #[error("IO error")]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error("failed to allocate texture space for image")]
    Allocation,
}

#[cfg(feature = "image")]
impl From<kas::draw::AllocError> for ImageError {
    fn from(_: kas::draw::AllocError) -> ImageError {
        ImageError::Allocation
    }
}

#[impl_self]
mod Image {
    /// A raster iamge
    ///
    /// May be default constructed (result is empty).
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct Image {
        core: widget_core!(),
        scaling: Scaling,
        handle: Option<ImageHandle>,
    }

    impl Self {
        /// Construct from a pre-allocated image
        ///
        /// The image may be allocated through the [`DrawShared`] interface.
        #[inline]
        pub fn new(handle: ImageHandle, draw: &mut dyn DrawShared) -> Option<Self> {
            draw.image_size(&handle).map(|size| {
                let mut sprite = Self::default();
                sprite.scaling.size = size.cast();
                sprite.handle = Some(handle);
                sprite
            })
        }

        /// Construct from a path
        #[cfg(feature = "image")]
        #[inline]
        pub fn new_path<P: AsRef<std::path::Path>>(
            path: P,
            draw: &mut dyn DrawShared,
        ) -> Result<Self, ImageError> {
            let mut sprite = Self::default();
            sprite._load_path(path, draw)?;
            Ok(sprite)
        }

        /// Assign a pre-allocated image
        ///
        /// Returns `true` on success. On error, `self` is unchanged.
        pub fn set(
            &mut self,
            cx: &mut EventState,
            handle: ImageHandle,
            draw: &mut dyn DrawShared,
        ) -> bool {
            if let Some(size) = draw.image_size(&handle) {
                self.scaling.size = size.cast();
                self.handle = Some(handle);
                cx.resize(self);
                true
            } else {
                false
            }
        }

        /// Load from a path
        ///
        /// On error, `self` is unchanged.
        #[cfg(feature = "image")]
        pub fn load_path<P: AsRef<std::path::Path>>(
            &mut self,
            cx: &mut EventState,
            path: P,
            draw: &mut dyn DrawShared,
        ) -> Result<(), ImageError> {
            self._load_path(path, draw).map(|_| {
                cx.resize(self);
            })
        }

        #[cfg(feature = "image")]
        fn _load_path<P: AsRef<std::path::Path>>(
            &mut self,
            path: P,
            draw: &mut dyn DrawShared,
        ) -> Result<(), ImageError> {
            let image = image::ImageReader::open(path)?
                .with_guessed_format()?
                .decode()?;

            // TODO(opt): we convert to RGBA8 since this is the only format common
            // to both the image and wgpu crates. It may not be optimal however.
            // It also assumes that the image colour space is sRGB.
            let image = image.into_rgba8();
            let size = image.dimensions();

            let handle = draw.image_alloc(size)?;
            draw.image_upload(&handle, &image, kas::draw::ImageFormat::Rgba8);

            if let Some(old_handle) = self.handle.take() {
                draw.image_free(old_handle);
            }

            self.scaling.size = size.cast();
            self.handle = Some(handle);

            Ok(())
        }

        /// Remove image (set empty)
        pub fn clear(&mut self, cx: &mut EventState, draw: &mut dyn DrawShared) {
            if let Some(handle) = self.handle.take() {
                draw.image_free(handle);
                cx.resize(self);
            }
        }

        /// Set size in logical pixels
        pub fn set_logical_size(&mut self, size: impl Into<LogicalSize>) {
            self.scaling.size = size.into();
        }

        /// Set size in logical pixels (inline)
        #[must_use]
        pub fn with_logical_size(mut self, size: impl Into<LogicalSize>) -> Self {
            self.scaling.size = size.into();
            self
        }

        /// Set the margin style (inline)
        ///
        /// By default, this is [`MarginStyle::Large`].
        #[must_use]
        #[inline]
        pub fn with_margin_style(mut self, style: MarginStyle) -> Self {
            self.scaling.margins = style;
            self
        }

        /// Control whether the aspect ratio is fixed (inline)
        ///
        /// By default this is fixed.
        #[must_use]
        #[inline]
        pub fn with_fixed_aspect_ratio(mut self, fixed: bool) -> Self {
            self.scaling.fix_aspect = fixed;
            self
        }

        /// Set the stretch factor (inline)
        ///
        /// By default this is [`Stretch::None`]. Particular to this widget,
        /// [`Stretch::None`] will avoid stretching of content, aligning instead.
        #[must_use]
        #[inline]
        pub fn with_stretch(mut self, stretch: Stretch) -> Self {
            self.scaling.stretch = stretch;
            self
        }
    }

    impl Layout for Image {
        fn rect(&self) -> Rect {
            self.scaling.rect
        }

        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            self.scaling.size_rules(sizer, axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_default();
            let scale_factor = cx.size_cx().scale_factor();
            self.scaling.set_rect(rect, align, scale_factor);
        }

        fn draw(&self, mut draw: DrawCx) {
            if let Some(id) = self.handle.as_ref().map(|h| h.id()) {
                draw.image(self.rect(), id);
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Image
        }
    }
}
