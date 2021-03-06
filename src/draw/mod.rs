// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! # Draw APIs
//!
//! Multiple drawing APIs are available. Each has a slightly different purpose.
//!
//! ### High-level themeable interface
//!
//! When widgets are sized or drawn, they are provided a [`SizeHandle`] or a
//! [`DrawHandle`] trait object. A [`SizeHandle`] may also be obtained through
//! [`kas::event::Manager::size_handle`].
//!
//! These traits are implemented by the theme of choice, providing a high-level
//! themed API over "widget features".
//!
//! [`SizeHandle`] is the only part of the API providing sizing data. If drawing
//! via a lower-level API, it may still be necessary to query the scale factor
//! or some feature size via [`SizeHandle`].
//!
//! ### Medium-level drawing interfaces
//!
//! The theme draws widget components over a [`Draw`] object (unique to the
//! current draw context) plus a reference to [`DrawShared`] (for shared data
//! related to drawing, e.g. loaded images). Widgets may access this same API
//! via [`DrawHandle::draw_device`].
//!
//! Both [`Draw`] and [`DrawShared`] are wrappers over types provided by the
//! shell implementing [`Drawable`] and [`DrawableShared`] respectively.
//! Extension traits to [`Drawable`] (which may be defined elsewhere) cover
//! further functionality.
//!
//! ### Low-level interface
//!
//! There is no universal graphics API, hence none is provided by this crate.
//! Instead, shells may provide their own extensions allowing direct access
//! to the host graphics API, for example
//! [`kas-wgpu::draw::CustomPipe`](https://docs.rs/kas-wgpu/*/kas_wgpu/draw/trait.CustomPipe.html).
//! The `mandlebrot` example demonstrates use of a custom draw pipe.
//!
//! ## Draw order
//!
//! All draw operations may be batched, thus where draw operations overlap the
//! result depends on the order batches are executed. This is expected to be in
//! the following order:
//!
//! 1.  Images
//! 2.  Non-rounded primitives (e.g. [`Draw::rect`])
//! 3.  Rounded primitives (e.g. [`Draw::rounded_line`])
//! 4.  Custom draw routines (`CustomPipe`)
//! 5.  Text
//!
//! Note that clip regions are always drawn after their parent region, thus
//! one can use [`Draw::new_clip_region`] to control draw order. This is
//! demonstrated in the `clock` example.

pub mod color;

#[allow(clippy::module_inception)]
mod draw;
mod draw_shared;
mod handle;
mod images;
mod theme;

use crate::cast::Cast;

pub use draw::*;
pub use draw_shared::{DrawShared, DrawSharedT, DrawableShared};
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
