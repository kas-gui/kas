// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! The drawing API is tiered: add bounds on the traits you need.
//!
//! All drawing operations are batched and do not happen immediately.
//! The order of draw operations is as follows:
//!
//! 1. [`DrawFlat`] - for "flat" (non-bevelled) shading; this may or may not
//!     be combined with [`DrawSquare`] rendering
//! 2. [`DrawSquare`] - for bevelled shading with square edges
//! 3. [`DrawRound`] - for bevelled shading with rounded edges; this happens
//!     after flat and square rendering and may not draw all pixels within rect
//!     bounds
//! 4. [`DrawText`] - for text rendering; this happens after all other rendering

mod draw_pipe;
mod round_pipe;
mod square_pipe;

pub use draw_pipe::{DrawPipe, DrawText};

pub fn read_glsl(code: &str, stage: glsl_to_spirv::ShaderType) -> Vec<u32> {
    wgpu::read_spirv(glsl_to_spirv::compile(&code, stage).unwrap()).unwrap()
}

/// 3-part colour data
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl From<kas::draw::Colour> for Rgb {
    fn from(c: kas::draw::Colour) -> Self {
        Rgb {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}
