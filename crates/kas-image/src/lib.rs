// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS resvg & tiny-skia integration
//!
//! This crate provides [`Svg`] and [`Canvas`] widgets using the [tiny-skia] and
//! [resvg] libraries by [Yevhenii Reizner "RazrFalcon"](https://github.com/RazrFalcon/).
//!
//! [tiny-skia]: https://crates.io/crates/tiny-skia
//! [resvg]: https://crates.io/crates/resvg

pub extern crate tiny_skia;

#[cfg(feature = "canvas")] mod canvas;
#[cfg(feature = "image")] mod image;
#[cfg(feature = "svg")] mod svg;

#[cfg(feature = "canvas")]
pub use canvas::{Canvas, CanvasProgram};
#[cfg(feature = "image")]
pub use image::{Image, ImageError};
#[cfg(feature = "svg")] pub use svg::Svg;

use kas::cast::{Conv, ConvFloat};
use kas::geom::{Rect, Vec2};
use kas::impl_scope;
use kas::layout::{AlignPair, AxisInfo, LogicalSize, SizeRules, Stretch};
use kas::theme::{MarginStyle, SizeCx};

/// Load a window icon from a path
#[cfg(feature = "image")]
pub fn window_icon_from_path<P: AsRef<std::path::Path>>(
    path: P,
) -> Result<kas::window::Icon, Box<dyn std::error::Error>> {
    // TODO(opt): image loading could be de-duplicated with
    // DrawShared::image_from_path, but this may not be worthwhile.
    let im = ::image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    let (w, h) = im.dimensions();
    Ok(kas::window::Icon::from_rgba(im.into_vec(), w, h)?)
}

impl_scope! {
    /// Control over image scaling
    #[impl_default]
    #[derive(Clone, Debug, PartialEq)]
    struct Scaling {
        /// Display size (logical pixels)
        ///
        /// This may be set by the providing type or by the user.
        pub size: LogicalSize,
        /// Margins
        pub margins: MarginStyle,
        /// If true, aspect ratio is fixed relative to [`Self::size`]
        ///
        /// Default: `true`
        pub fix_aspect: bool = true,
        /// Widget stretchiness
        ///
        /// If is `None`, max size is limited to ideal size.
        ///
        /// By default, this is `None`.
        pub stretch: Stretch,
    }
}

impl Scaling {
    /// Generates [`SizeRules`] based on size
    pub fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
        let scale_factor = sizer.scale_factor();
        let ideal = self.size.to_physical(scale_factor).extract(axis);
        SizeRules::new(ideal, ideal, self.stretch)
            .with_margins(sizer.margins(self.margins).extract(axis))
    }

    /// Constrains and aligns within the given `rect`
    ///
    /// This aligns content when using [`Stretch::None`] and when fixed-aspect
    /// scaling constrains size.
    pub fn align(&mut self, rect: Rect, align: AlignPair, scale_factor: f32) -> Rect {
        let mut size = rect.size;

        if self.stretch == Stretch::None {
            let ideal = self.size.to_physical(scale_factor);
            size = size.min(ideal);
        }

        if self.fix_aspect {
            let logical_size = Vec2::from(self.size);
            let Vec2(rw, rh) = Vec2::conv(size) / logical_size;

            // Use smaller ratio, if any is finite
            if rw < rh {
                size.1 = i32::conv_nearest(rw * logical_size.1);
            } else if rh < rw {
                size.0 = i32::conv_nearest(rh * logical_size.0);
            }
        }

        align.aligned_rect(size, rect)
    }
}
