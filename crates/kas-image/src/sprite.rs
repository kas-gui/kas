// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! 2D pixmap widget

use super::Scaling;
use kas::draw::ImageHandle;
use kas::layout::LogicalSize;
use kas::prelude::*;
use kas::theme::MarginStyle;

#[impl_self]
mod Sprite {
    /// A raster image widget, loaded from a handle
    ///
    /// Size is inferred from the loaded image. By default, scaling is limited
    /// to integer multiples of the source image size.
    ///
    /// May be default constructed (result is empty).
    #[derive(Clone, Debug, Default)]
    #[widget]
    pub struct Sprite {
        core: widget_core!(),
        scaling: Scaling,
        image_size: Size,
        handle: Option<ImageHandle>,
    }

    impl Self {
        /// Construct an empty (unallocated) image
        #[inline]
        pub fn new() -> Self {
            Self::default()
        }

        /// Assign a pre-allocated image
        ///
        /// Returns `true` on success. On error, `self` is unchanged.
        pub fn set(&mut self, cx: &mut EventCx, handle: ImageHandle) -> bool {
            let draw = cx.draw_shared();
            if let Some(old_handle) = self.handle.take() {
                draw.image_free(old_handle);
            }

            if let Some(size) = draw.image_size(&handle) {
                if self.scaling.size == LogicalSize::default() && self.image_size != size {
                    cx.resize(&self);
                }
                self.image_size = size;
                self.handle = Some(handle);
                true
            } else {
                self.image_size = Size::ZERO;
                false
            }
        }

        /// Remove image (set empty)
        pub fn clear(&mut self, cx: &mut EventCx) {
            if let Some(handle) = self.handle.take() {
                cx.draw_shared().image_free(handle);
                if self.scaling.size == LogicalSize::default() {
                    cx.resize(self);
                }
            }
        }

        /// Set size in logical pixels
        ///
        /// This enables fractional scaling of the image with a fixed aspect ratio.
        pub fn set_logical_size(&mut self, size: impl Into<LogicalSize>) {
            self.scaling.size = size.into();
        }

        /// Set size in logical pixels (inline)
        ///
        /// This enables fractional scaling of the image with a fixed aspect ratio.
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
        /// This is only applicable when using fractional scaling (see
        /// [`Self::set_logical_size`]) since integer scaling always uses a
        /// fixed aspect ratio. By default this is enabled.
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

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            if self.scaling.size == LogicalSize::default() {
                let scale: i32 = (cx.scale_factor() * 0.9).cast_nearest();
                debug_assert!(scale >= 1);
                SizeRules::fixed(self.image_size.extract(axis) * scale)
                    .with_margins(cx.margins(self.scaling.margins).extract(axis))
            } else {
                self.scaling.size_rules(cx, axis)
            }
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = hints.complete_default();
            let rect = if self.scaling.size == LogicalSize::default() {
                // Avoid divide-by-zero
                let image_size = self.image_size.max(Size::splat(1));
                let scale = (rect.size.0 / image_size.0)
                    .min(rect.size.1 / image_size.1)
                    .max(1);
                let size = self.image_size * scale;
                align.aligned_rect(size, rect)
            } else {
                let scale_factor = cx.size_cx().scale_factor();
                self.scaling.align(rect, align, scale_factor)
            };
            widget_set_rect!(rect);
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
