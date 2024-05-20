// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — draw rounded

use super::color::Rgba;
use super::{Draw, DrawIface, DrawImpl, DrawSharedImpl, PassId};
use crate::geom::{Quad, Vec2};

/// Extended draw interface for [`DrawIface`] providing rounded drawing
///
/// All methods draw some feature.
pub trait DrawRounded: Draw {
    /// Draw a line with rounded ends and uniform colour
    ///
    /// This command draws a line segment between the points `p1` and `p2`.
    /// Pixels within the given `radius` of this segment are drawn, resulting
    /// in rounded ends and width `2 * radius`.
    ///
    /// Note that for rectangular, axis-aligned lines, [`DrawImpl::rect`] should be
    /// preferred.
    fn rounded_line(&mut self, p1: Vec2, p2: Vec2, radius: f32, col: Rgba);

    /// Draw a circle or oval of uniform colour
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// The `inner_radius` parameter gives the inner radius relative to the
    /// outer radius: a value of `0.0` will result in the whole shape being
    /// painted, while `1.0` will result in a zero-width line on the outer edge.
    fn circle(&mut self, rect: Quad, inner_radius: f32, col: Rgba);

    /// Draw a circle or oval with two colours
    ///
    /// More generally, this shape is an axis-aligned oval which may be hollow.
    ///
    /// Colour `col1` is used at the centre and `col2` at the edge with linear
    /// blending. The edge is not anti-aliased.
    ///
    /// Note: this is drawn *before* other drawables, allowing it to be used
    /// for shadows without masking.
    fn circle_2col(&mut self, rect: Quad, col1: Rgba, col2: Rgba);

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
    fn rounded_frame(&mut self, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba);

    /// Draw a frame with rounded corners with two colours
    ///
    /// This is a variant of `rounded_frame` which blends between two colours,
    /// `c1` at the inner edge and `c2` at the outer edge.
    ///
    /// Note: this is drawn *before* other drawables, allowing it to be used
    /// for shadows without masking.
    fn rounded_frame_2col(&mut self, outer: Quad, inner: Quad, c1: Rgba, c2: Rgba);
}

impl<'a, DS: DrawSharedImpl> DrawRounded for DrawIface<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    #[inline]
    fn rounded_line(&mut self, p1: Vec2, p2: Vec2, radius: f32, col: Rgba) {
        self.draw.rounded_line(self.pass, p1, p2, radius, col);
    }
    #[inline]
    fn circle(&mut self, rect: Quad, inner_radius: f32, col: Rgba) {
        self.draw.circle(self.pass, rect, inner_radius, col);
    }
    #[inline]
    fn circle_2col(&mut self, rect: Quad, col1: Rgba, col2: Rgba) {
        self.draw.circle_2col(self.pass, rect, col1, col2);
    }
    #[inline]
    fn rounded_frame(&mut self, outer: Quad, inner: Quad, inner_radius: f32, col: Rgba) {
        self.draw
            .rounded_frame(self.pass, outer, inner, inner_radius, col);
    }
    #[inline]
    fn rounded_frame_2col(&mut self, outer: Quad, inner: Quad, c1: Rgba, c2: Rgba) {
        self.draw
            .rounded_frame_2col(self.pass, outer, inner, c1, c2);
    }
}

/// Implementation target for [`DrawRounded`]
///
/// This trait is an extension over [`DrawImpl`] providing rounded shapes.
///
/// The primitives provided by this trait are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
pub trait DrawRoundedImpl: DrawImpl {
    /// Draw a line with rounded ends and uniform colour
    fn rounded_line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, radius: f32, col: Rgba);

    /// Draw a circle or oval of uniform colour
    fn circle(&mut self, pass: PassId, rect: Quad, inner_radius: f32, col: Rgba);

    /// Draw a circle or oval with two colours
    fn circle_2col(&mut self, pass: PassId, rect: Quad, col1: Rgba, col2: Rgba);

    /// Draw a frame with rounded corners and uniform colour
    fn rounded_frame(&mut self, pass: PassId, outer: Quad, inner: Quad, r1: f32, col: Rgba);

    /// Draw a frame with rounded corners with two colours
    fn rounded_frame_2col(&mut self, pass: PassId, outer: Quad, inner: Quad, c1: Rgba, c2: Rgba);
}
