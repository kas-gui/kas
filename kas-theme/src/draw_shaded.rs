// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing APIs — shaded drawing

use kas::draw::color::Rgba;
use kas::draw::{DrawIface, Drawable, DrawableShared, PassId};
use kas::geom::Quad;

/// Extension trait providing shaded drawing over [`DrawIface`]
pub trait DrawableShadedExt {
    /// Add a shaded square to the draw buffer
    fn shaded_square(&mut self, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a shaded circle to the draw buffer
    fn shaded_circle(&mut self, rect: Quad, norm: (f32, f32), col: Rgba);

    /// Add a square shaded frame to the draw buffer.
    fn shaded_square_frame(
        &mut self,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    );

    /// Add a rounded shaded frame to the draw buffer.
    fn shaded_round_frame(&mut self, outer: Quad, inner: Quad, norm: (f32, f32), col: Rgba);
}

impl<'a, DS: DrawableShared> DrawableShadedExt for DrawIface<'a, DS>
where
    DS::Draw: DrawableShaded,
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

/// Drawing commands for shaded shapes
///
/// This trait is an extension over [`Drawable`] providing solid shaded shapes.
///
/// Some drawing primitives (the "round" ones) are partially transparent.
/// If the implementation buffers draw commands, it should draw these
/// primitives after solid primitives.
///
/// These are parameterised via a pair of normals, `(inner, outer)`. These may
/// have values from the closed range `[-1, 1]`, where -1 points inwards,
/// 0 is perpendicular to the screen towards the viewer, and 1 points outwards.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait DrawableShaded: Drawable {
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
