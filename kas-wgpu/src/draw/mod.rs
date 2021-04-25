// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`
//!
//! Extensions to the API of [`kas::draw`], plus some utility types.

mod atlases;
mod common;
mod custom;
mod draw_pipe;
mod draw_text;
mod flat_round;
mod images;
mod shaded_round;
mod shaded_square;
mod shaders;
mod text_pipe;

use kas::geom::Rect;
use wgpu::TextureFormat;

pub(crate) use images::ImageError;
pub(crate) use shaders::ShaderManager;

pub use custom::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom};

pub(crate) const TEX_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

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

/// Shared pipeline data
pub struct DrawPipe<C> {
    local_pool: futures::executor::LocalPool,
    staging_belt: wgpu::util::StagingBelt,
    bgl_common: wgpu::BindGroupLayout,
    images: images::Images,
    shaded_square: shaded_square::Pipeline,
    shaded_round: shaded_round::Pipeline,
    flat_round: flat_round::Pipeline,
    custom: C,
    pub(crate) text: text_pipe::Pipeline,
}

/// Per-window pipeline data
pub struct DrawWindow<CW: CustomWindow> {
    scale_buf: wgpu::Buffer,
    clip_regions: Vec<Rect>,
    bg_common: wgpu::BindGroup,
    images: images::Window,
    shaded_square: shaded_square::Window,
    shaded_round: shaded_round::Window,
    flat_round: flat_round::Window,
    custom: CW,
    pub(crate) text: text_pipe::Window,
}
