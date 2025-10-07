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

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub extern crate tiny_skia;

#[cfg(feature = "canvas")] mod canvas;
#[cfg(feature = "image")] mod image;
#[cfg(feature = "svg")] mod svg;

#[cfg(feature = "canvas")]
pub use canvas::{Canvas, CanvasProgram};
#[cfg(feature = "image")]
pub use image::{Image, ImageError};
#[cfg(feature = "svg")] pub use svg::Svg;

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
