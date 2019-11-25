// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! TODO: move traits up to kas?
//!
//! The drawing API is tiered: add bounds on the traits you need.
//!
//! All drawing operations are batched and do not happen immediately.
//! The order of draw operations is:
//!
//! 1. [`DrawFlat`] - "flat" (non-bevelled) shading
//! 2. [`DrawSquare`] - bevelled shading with square edges
//! 3. [`DrawRound`] - bevelled shading with rounded edges
//! 4. [`DrawText`] - text rendering

use kas::geom::Size;

use crate::colour::Colour;
use crate::round_pipe::Rounded;
use crate::tri_pipe::TriPipe;
use crate::vertex::Vec2;

/// Abstraction over drawing commands
pub trait Draw {
    /// Add a rectangle to the draw buffer defined by two corners
    /// `aa` and `bb` with solid colour `col`.
    fn draw_flat_quad(&mut self, aa: Vec2, bb: Vec2, col: Colour);

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd`, with solid colour `col`.
    ///
    /// The frame is shaded according to its normal, and appears flat when
    /// `norm = (0.0, 0.0)`.
    ///
    /// The normal is calculated from the `x` component (for verticals) or `y`
    /// component (for horizontals); the other `x` / `y` component is set to
    /// zero while the `z` component is calculated such that `x² + y² + z² = 1`.
    ///
    /// The normal component itself is calculated via linear interpolation
    /// between `outer` and `inner`, where parameter `norm = (outer, inner)`,
    /// with both parameters pointing towards the centre of the frame (thus
    /// positive values make the frame appear sunken).
    ///
    /// Component-wise bounds: `aa < cc < dd < bb`; `-1 < norm < 1`.
    fn draw_square_frame(
        &mut self,
        aa: Vec2,
        bb: Vec2,
        cc: Vec2,
        dd: Vec2,
        norm: (f32, f32),
        col: Colour,
    );

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colour `col`.
    // TODO: allow control of normals
    fn draw_round_frame(&mut self, aa: Vec2, bb: Vec2, cc: Vec2, dd: Vec2, col: Colour);
}

/// Manager of draw pipes and implementor of [`Draw`]
pub struct DrawPipe {
    round_pipe: Rounded,
    tri_pipe: TriPipe,
}

impl DrawPipe {
    /// Construct
    pub fn new(device: &wgpu::Device, size: Size) -> Self {
        DrawPipe {
            tri_pipe: TriPipe::new(device, size),
            round_pipe: Rounded::new(device, size),
        }
    }

    /// Process window resize
    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.tri_pipe.resize(device, &mut encoder, size);
        self.round_pipe.resize(device, &mut encoder, size);
        encoder.finish()
    }

    /// Render batched draw instructions via `rpass`
    pub fn render(&mut self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass) {
        self.tri_pipe.render(device, rpass);
        self.round_pipe.render(device, rpass);
    }
}

impl Draw for DrawPipe {
    fn draw_flat_quad(&mut self, aa: Vec2, bb: Vec2, col: Colour) {
        self.tri_pipe.add_quad(aa, bb, col)
    }

    fn draw_square_frame(
        &mut self,
        aa: Vec2,
        bb: Vec2,
        cc: Vec2,
        dd: Vec2,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.tri_pipe.add_frame(aa, bb, cc, dd, norm, col)
    }

    fn draw_round_frame(&mut self, aa: Vec2, bb: Vec2, cc: Vec2, dd: Vec2, col: Colour) {
        self.round_pipe.add_frame(aa, bb, cc, dd, col)
    }
}
