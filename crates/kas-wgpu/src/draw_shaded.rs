// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shaded drawing

use kas::draw::color::Rgba;
use kas::draw::{DrawIface, DrawImpl, DrawSharedImpl, PassId};
use kas::geom::Quad;

/// Extension trait providing shaded drawing for [`DrawIface`]
///
/// All methods draw some feature.
///
/// Methods are parameterised via a pair of normals, `(inner, outer)`, which
/// specify the surface normal direction at inner and outer edges of the feature
/// respectively (with interpolation between these edges). These have values
/// from the closed range `[-1, 1]`, where -1 points towards the inside of the
/// feature, 1 points away from the feature, and 0 is perpendicular to the
/// screen towards the viewer.
pub trait DrawShaded {
    /// Add a shaded square to the draw buffer
    ///
    /// For shading purposes, the mid-point is considered the inner edge.
    fn shaded_square(&mut self, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a shaded circle to the draw buffer
    ///
    /// For shading purposes, the mid-point is considered the inner edge.
    fn shaded_circle(&mut self, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a shaded frame with square corners to the draw buffer
    fn shaded_square_frame(
        &mut self,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    );

    /// Add a shaded frame with rounded corners to the draw buffer
    fn shaded_round_frame(&mut self, outer: Quad, inner: Quad, norm: (f32, f32), col: Rgba);
}

impl<'a, DS: DrawSharedImpl> DrawShaded for DrawIface<'a, DS>
where
    DS::Draw: DrawShadedImpl,
{
    fn shaded_square(&mut self, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.draw.shaded_square(self.pass, rect, norm, col);
    }

    fn shaded_circle(&mut self, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.draw.shaded_circle(self.pass, rect, norm, col);
    }

    fn shaded_square_frame(
        &mut self,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    ) {
        self.draw
            .shaded_square_frame(self.pass, outer, inner, norm, outer_col, inner_col);
    }

    fn shaded_round_frame(&mut self, outer: Quad, inner: Quad, norm: (f32, f32), col: Rgba) {
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);
    }
}

/// Extended draw interface for [`DrawIface`] providing shaded drawing
///
/// This trait is an extension over [`DrawImpl`] providing solid shaded shapes.
///
/// Some drawing primitives (the "round" ones) are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
///
/// Methods are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
pub trait DrawShadedImpl: DrawImpl {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, pass: PassId, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, pass: PassId, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        pass: PassId,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(
        &mut self,
        pass: PassId,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Rgba,
    );
}
