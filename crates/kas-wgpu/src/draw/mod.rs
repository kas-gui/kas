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
mod flat_round;
mod images;
mod round_2col;
mod shaded_round;
mod shaded_square;
mod shaders;
mod text_pipe;

use kas::draw::WindowCommon;
use kas::geom::{Offset, Rect};
use shaders::ShaderManager;
use wgpu::TextureFormat;

pub use custom::{CustomPipe, CustomPipeBuilder, CustomWindow, DrawCustom};

/// Output format
///
/// Required by WGPU to be BGRA, either sRGB or Unorm. Currently we assume sRGB
/// and let the graphics pipeline handle colour conversions.
pub(crate) const RENDER_TEX_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

type Scale = [f32; 4];

/// Shared pipeline data
pub struct DrawPipe<C> {
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    queue: wgpu::Queue,
    staging_belt: wgpu::util::StagingBelt,
    bgl_common: wgpu::BindGroupLayout,
    light_norm_buf: wgpu::Buffer,
    bg_common: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    images: images::Images,
    shaded_square: shaded_square::Pipeline,
    shaded_round: shaded_round::Pipeline,
    flat_round: flat_round::Pipeline,
    round_2col: round_2col::Pipeline,
    custom: C,
    pub(crate) text: text_pipe::Pipeline,
}

kas::impl_scope! {
    /// Per-window pipeline data
    #[impl_default(where CW: Default)]
    pub struct DrawWindow<CW: CustomWindow> {
        pub(crate) common: WindowCommon,
        scale: Scale,
        clip_regions: Vec<(Rect, Offset)> = vec![Default::default()],
        images: images::Window,
        shaded_square: shaded_square::Window,
        shaded_round: shaded_round::Window,
        flat_round: flat_round::Window,
        round_2col: round_2col::Window,
        custom: CW,
        pub(crate) text: text_pipe::Window,
    }
}
