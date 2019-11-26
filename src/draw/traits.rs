// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API

use super::{Colour, Quad, Vec2};

/// Abstraction over flat drawing commands
pub trait DrawFlat {
    /// Add a rectangle to the draw buffer.
    ///
    /// Expected componentwise bounds on input: `q.0 < q.1`.
    /// Failure to meet bounds may lead to graphical tweaks or no drawing.
    fn draw_flat_quad(&mut self, quad: Quad, col: Colour);
}

/// Abstraction over square drawing commands
pub trait DrawSquare {
    /// Add a frame to the buffer.
    ///
    /// The frame has square corners and is shaded according to its normal.
    /// Frame sides are divided at the corners by a straight line from inner to
    /// outer corner. The frame appears flat when `norm = (0.0, 0.0)`.
    ///
    /// The normal is calculated from the `x` component (for verticals) or `y`
    /// component (for horizontals); the other `x` / `y` component is set to
    /// zero while the `z` component is calculated such that `x² + y² + z² = 1`.
    ///
    /// The normal component itself is calculated via linear interpolation
    /// between `outer` and `inner`, where parameter `norm = (outer, inner)`,
    /// with both parameters pointing out from the frame (thus
    /// positive values make the frame appear raised).
    ///
    /// Expected componentwise bounds on input:
    /// `outer.0 < inner.0 < inner.1 < outer.1` and `-1 ≤ norm ≤ 1`.
    /// Failure to meet bounds may lead to graphical tweaks or no drawing.
    fn draw_square_frame(&mut self, outer: Quad, inner: Quad, norm: Vec2, col: Colour);
}

/// Abstraction over rounded drawing commands
pub trait DrawRound {
    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd`, with solid colour `col`.
    ///
    /// The frame has rounded corners and is shaded according to its normal.
    /// Corners are smoothly shaded; pixels beyond the outer curve are not
    /// drawn. The frame appears flat when `norm = (0.0, 0.0)`.
    ///
    /// The normal is calculated from the `x` component (for verticals) or `y`
    /// component (for horizontals); the other `x` / `y` component is set to
    /// zero while the `z` component is calculated such that `x² + y² + z² = 1`.
    ///
    /// The normal component itself is calculated via linear interpolation
    /// between `outer` and `inner`, where parameter `norm = (outer, inner)`,
    /// with both parameters pointing out from the frame (thus
    /// positive values make the frame appear raised).
    ///
    /// Expected componentwise bounds on input:
    /// `outer.0 < inner.0 < inner.1 < outer.1` and `-1 ≤ norm ≤ 1`.
    /// Failure to meet bounds may lead to graphical tweaks or no drawing.
    fn draw_round_frame(&mut self, outer: Quad, inner: Quad, norm: Vec2, col: Colour);
}
