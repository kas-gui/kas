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

mod colour;
mod handle;
mod theme;

use std::any::Any;
use std::num::NonZeroU32;
use std::path::Path;

use crate::cast::Cast;
use crate::geom::{Quad, Rect, Size, Vec2};
use crate::text::{Effect, TextDisplay};

pub use colour::Colour;
pub use handle::*;
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

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageId(NonZeroU32);

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

/// Bounds on type shared across [`Draw`] implementations
pub trait DrawShared: 'static {
    type Draw: Draw;

    /// Load an image from a path, autodetecting file type
    ///
    /// TODO: revise error handling?
    fn load_image(&mut self, path: &Path) -> Result<ImageId, Box<dyn std::error::Error + 'static>>;

    /// Free an image
    fn remove_image(&mut self, id: ImageId);

    /// Get size of image
    fn image_size(&self, id: ImageId) -> Option<Size>;

    /// Draw the image in the given `rect`
    fn draw_image(&self, window: &mut Self::Draw, pass: Pass, id: ImageId, rect: Quad);

    /// Draw text with a colour
    fn draw_text(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Colour,
    );

    /// Draw text with a colour and effects
    ///
    /// The effects list does not contain colour information, but may contain
    /// underlining/strikethrough information. It may be empty.
    fn draw_text_col_effects(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        col: Colour,
        effects: &[Effect<()>],
    );

    /// Draw text with effects
    ///
    /// The `effects` list provides both underlining and colour information.
    /// If the `effects` list is empty or the first entry has `start > 0`, a
    /// default entity will be assumed.
    fn draw_text_effects(
        &mut self,
        window: &mut Self::Draw,
        pass: Pass,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Colour>],
    );
}

/// Base abstraction over drawing
///
/// Unlike [`DrawHandle`], coordinates are specified via a [`Vec2`] and
/// rectangular regions via [`Quad`]. The same coordinate system is used, hence
/// type conversions can be performed with `from` and `into`. Integral
/// coordinates align with pixels, non-integral coordinates may also be used.
///
/// All draw operations may be batched; when drawn primitives overlap, the
/// results are only loosely defined. Draw operations involving transparency
/// should be ordered after those without transparency.
///
/// Draw operations take place over multiple render passes, identified by a
/// handle of type [`Pass`]. In general the user only needs to pass this value
/// into methods as required. [`Draw::add_clip_region`] creates a new [`Pass`].
pub trait Draw: Any {
    /// Cast self to [`std::any::Any`] reference.
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// shell-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Add a clip region
    ///
    /// Clip regions are cleared each frame and so must be recreated on demand.
    fn add_clip_region(&mut self, rect: Rect) -> Pass;

    /// Draw a rectangle of uniform colour
    fn rect(&mut self, pass: Pass, rect: Quad, col: Colour);

    /// Draw a frame of uniform colour
    ///
    /// The frame is defined by the area inside `outer` and not inside `inner`.
    fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Colour);
}

/// Drawing commands for rounded shapes
///
/// This trait is an extension over [`Draw`] providing rounded shapes.
///
/// The primitives provided by this trait are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
pub trait DrawRounded: Draw {
    /// Draw a line with rounded ends and uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    /// Pixels within the given `radius` of this segment are drawn, resulting
    /// in rounded ends and width `2 * radius`.
    ///
    /// Note that for rectangular, axis-aligned lines, [`Draw::rect`] should be
    /// preferred.
    fn rounded_line(&mut self, pass: Pass, p1: Vec2, p2: Vec2, radius: f32, col: Colour);

    /// Draw a circle or oval of uniform colour
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn circle(&mut self, pass: Pass, rect: Quad, inner_radius: f32, col: Colour);

    /// Draw a frame with rounded corners and uniform colour
    ///
    /// All drawing occurs within the `outer` rect and outside of the `inner`
    /// rect. Corners are circular (or more generally, ovular), centered on the
    /// inner corners.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    /// When `inner_radius > 0`, the frame will be visually thinner than the
    /// allocated area.
    fn rounded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        inner_radius: f32,
        col: Colour,
    );
}

/// Drawing commands for shaded shapes
///
/// This trait is an extension over [`Draw`] providing solid shaded shapes.
///
/// Some drawing primitives (the "round" ones) are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
///
/// These are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
pub trait DrawShaded: Draw {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Colour);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Colour);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Colour,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Colour,
    );
}
