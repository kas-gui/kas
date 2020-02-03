// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! The drawing API is tiered: add bounds on the traits you need.
//!
//! All drawing operations are batched and do not happen immediately.

mod draw_pipe;
mod round_pipe;
mod shaders;
mod square_pipe;
mod vector;

pub use kas::draw::{Colour, Draw};

pub use draw_pipe::{DrawPipe, DrawShaded, DrawText, ShadeStyle};
pub(crate) use shaders::ShaderManager;
pub use vector::{Quad, Vec2};

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
