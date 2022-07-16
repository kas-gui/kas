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

#![cfg_attr(doc_cfg, feature(doc_cfg))]

pub use tiny_skia;

mod canvas;
pub use canvas::{Canvas, CanvasProgram};

#[cfg(feature = "svg")]
mod svg;
#[cfg(feature = "svg")]
pub use svg::Svg;
