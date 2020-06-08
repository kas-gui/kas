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
//! [`DrawRounded`], [`DrawShaded`] and [`DrawText`] provide additional draw
//! routines. Toolkits are required to implement only the base [`Draw`] trait,
//! and may provide their own extension traits. For this reason, themes are
//! parameterised over an object `D: Draw + ...` (with specified trait bounds).
//!
//! The medium-level API will be extended in the future to support texturing
//! (not yet supported) and potentially a more comprehensive path-based API
//! (e.g. Lyon).
//!
//! ### Low-level interface
//!
//! There is no universal graphics API, hence none is provided by this crate.
//! Instead, toolkits may provide their own extensions allowing direct access
//! to the host graphics API, for example
//! [`kas-wgpu::draw::CustomPipe`](https://docs.rs/kas-wgpu/*/kas_wgpu/draw/trait.CustomPipe.html).

mod colour;
mod handle;
mod text;

use std::any::Any;

use crate::geom::{Quad, Rect, Vec2};

pub use colour::Colour;
pub use handle::{ClipRegion, DrawHandle, DrawHandleExt, InputState, SizeHandle, TextClass};
pub use text::{DrawText, DrawTextShared, FontId, PxScale, TextPart, TextProperties, TextSection};

/// Pass identifier
///
/// Users normally need only pass this value.
///
/// Custom render pipes should extract the pass number and depth value.
#[derive(Copy, Clone)]
pub struct Pass(u32, f32);

impl Pass {
    /// Construct a new pass from a `u32` identifier and depth value
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[inline]
    pub const fn new_pass_with_depth(n: u32, d: f32) -> Self {
        Pass(n, d)
    }

    /// The pass number
    ///
    /// This value is returned as `usize` but is always safe to store `as u32`.
    #[inline]
    pub fn pass(self) -> usize {
        self.0 as usize
    }

    /// The depth value
    #[inline]
    pub fn depth(self) -> f32 {
        self.1
    }
}

/// Bounds on type shared across [`Draw`] implementations
pub trait DrawShared {
    type Draw: Draw;
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
///
/// Each [`Pass`] has an associated depth value which may be used to determine
/// the result of overlapping draw commands.
pub trait Draw: Any {
    /// Cast self to [`std::any::Any`] reference.
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// toolkit-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Add a clip region
    ///
    /// Clip regions are cleared each frame and so must be recreated on demand.
    /// Each region has an associated depth value. The theme is responsible for
    /// assigning depth values.
    fn add_clip_region(&mut self, rect: Rect, depth: f32) -> Pass;

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
