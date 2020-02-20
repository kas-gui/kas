// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API
//!
//! This module includes abstractions over the drawing API and some associated
//! types.
//!
//! All draw operations are batched and do not happen immediately.

mod colour;

use std::any::Any;

use crate::geom::{Coord, Rect};
use crate::theme::TextProperties;

pub use colour::Colour;

/// Abstraction over drawing commands
///
/// Implementations may support drawing each feature with multiple styles, but
/// do not guarantee an exact match in each case.
///
/// Certain bounds on input are expected in each case. In case these are not met
/// the implementation may tweak parameters to ensure valid drawing. In the case
/// that the outer region does not have positive size or has reversed
/// coordinates, drawing may not occur at all.
pub trait Draw {
    /// Type returned by [`Draw::add_clip_region`].
    ///
    /// Supports [`Default`], which may be used to target the root region.
    type Region: Copy + Clone + Default;

    /// Cast self to [`std::any::Any`] reference.
    ///
    /// A downcast on this value may be used to obtain a reference to a
    /// toolkit-specific API.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Add a clip region
    ///
    /// Clip regions are cleared each frame and so must be recreated on demand.
    fn add_clip_region(&mut self, region: Rect) -> Self::Region;

    /// Add a line with rounded ends to the draw buffer
    ///
    /// Note that for rectangular, axis-aligned lines, [`Draw::rect`] should be
    /// preferred.
    fn rounded_line(
        &mut self,
        region: Self::Region,
        p1: Coord,
        p2: Coord,
        radius: f32,
        col: Colour,
    );

    /// Add a rectangle with flat shading to the draw buffer.
    fn rect(&mut self, region: Self::Region, rect: Rect, col: Colour);

    /// Add a flat circle to the draw buffer
    ///
    /// More generally, this shape is an axis-aligned oval.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn circle(&mut self, region: Self::Region, rect: Rect, inner_radius: f32, col: Colour);

    /// Add a frame with flat shading to the draw buffer.
    ///
    /// It is expected that the `outer` rect contains the `inner` rect.
    /// Failure may result in graphical glitches.
    fn frame(&mut self, region: Self::Region, outer: Rect, inner: Rect, col: Colour);

    /// Add a rounded flat frame to the draw buffer.
    ///
    /// All drawing occurs within the `outer` rect and outside of the `inner`
    /// rect. Corners are circular (or more generally, ovular), centered on the
    /// inner corners.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn rounded_frame(
        &mut self,
        region: Self::Region,
        outer: Rect,
        inner: Rect,
        inner_radius: f32,
        col: Colour,
    );
}

/// Drawing commands for shaded shapes
///
/// These are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
///
/// Shaded drawing may not be supported by all implementations.
pub trait DrawShaded: Draw {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, region: Self::Region, rect: Rect, norm: (f32, f32), col: Colour);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, region: Self::Region, rect: Rect, norm: (f32, f32), col: Colour);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        region: Self::Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(
        &mut self,
        region: Self::Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    );
}

/// Abstraction over text rendering
///
/// Note: the current API is designed to meet only current requirements since
/// changes are expected to support external font shaping libraries.
pub trait DrawText {
    /// Simple text drawing
    ///
    /// This allows text to be drawn according to a high-level API, and should
    /// satisfy most uses.
    fn text(&mut self, rect: Rect, text: &str, font_scale: f32, props: TextProperties, col: Colour);

    /// Calculate size bound on text
    ///
    /// This may be used with [`DrawText::text`] to calculate size requirements
    /// within [`kas::Layout::size_rules`].
    ///
    /// Bounds of `(f32::INFINITY, f32::INFINITY)` may be used if there are no
    /// constraints. This parameter allows forcing line-wrapping behaviour
    /// within the given bounds.
    fn text_bound(
        &mut self,
        text: &str,
        font_scale: f32,
        bounds: (f32, f32),
        line_wrap: bool,
    ) -> (f32, f32);
}
