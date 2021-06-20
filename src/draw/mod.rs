// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs
//!
//! Multiple drawing APIs are available. Each has a slightly different purpose.
//!
//! ### High-level themeable interface
//!
//! The [`DrawHandle`] trait and companion [`SizeHandle`] trait provide the
//! highest-level API over themeable widget components. These traits are
//! implemented by a theme defined in `kas-theme` or another crate.
//!
//! ### Medium-level drawing interfaces
//!
//! The [`Draw`] trait and its extensions are provided as the building-blocks
//! used to implement themes, but may also be used directly (as in the `clock`
//! example). These traits allow drawing of simple shapes, mostly in the form of
//! an axis-aligned box or frame with several shading options.
//!
//! The [`Draw`] trait itself contains very little; extension traits
//! [`DrawRounded`] and [`DrawShaded`] provide additional draw
//! routines. Shells are only required to implement the base [`Draw`] trait,
//! and may also provide their own extension traits. Themes may specify their
//! own requirements, e.g. `D: Draw + DrawRounded + DrawText`.
//!
//! The medium-level API will be extended in the future to support texturing
//! (not yet supported) and potentially a more comprehensive path-based API
//! (e.g. Lyon).
//!
//! ### Low-level interface
//!
//! There is no universal graphics API, hence none is provided by this crate.
//! Instead, shells may provide their own extensions allowing direct access
//! to the host graphics API, for example
//! [`kas-wgpu::draw::CustomPipe`](https://docs.rs/kas-wgpu/*/kas_wgpu/draw/trait.CustomPipe.html).

pub mod color;

mod draw;
mod draw_shared;
mod handle;
mod images;
mod theme;

use crate::cast::Cast;

pub use draw::*;
pub use draw_shared::{DrawShared, DrawableShared};
pub use handle::*;
pub use images::{ImageError, ImageFormat, ImageId};
pub use theme::*;

/// Pass identifier
///
/// Users normally need only pass this value.
///
/// Custom render pipes should extract the pass number.
#[derive(Copy, Clone)]
pub struct Pass(u32);

impl Pass {
    /// Construct a new pass from a `u32` identifier
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    #[inline]
    pub const fn new(n: u32) -> Self {
        Pass(n)
    }

    /// The pass number
    ///
    /// This value is returned as `usize` but is always safe to store `as u32`.
    #[inline]
    pub fn pass(self) -> usize {
        self.0.cast()
    }

    /// The depth value
    ///
    /// This is a historical left-over and always returns 0.0.
    #[inline]
    pub fn depth(self) -> f32 {
        0.0
    }
}
