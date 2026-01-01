// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use wgpu::{ShaderModule, include_spirv};

/// Shader manager
pub struct ShaderManager {
    pub vert_flat_round: ShaderModule,
    pub vert_round_2col: ShaderModule,
    pub vert_shaded_round: ShaderModule,
    pub vert_shaded_square: ShaderModule,
    pub vert_image: ShaderModule,
    pub vert_glyph: ShaderModule,
    pub frag_flat_round: ShaderModule,
    pub frag_round_2col: ShaderModule,
    pub frag_shaded_round: ShaderModule,
    pub frag_shaded_square: ShaderModule,
    pub frag_image: ShaderModule,
    pub frag_glyph: ShaderModule,
    pub frag_subpixel: ShaderModule,
}

macro_rules! create {
    ($device:ident, $path:expr) => {{ $device.create_shader_module(include_spirv!($path)) }};
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        ShaderManager {
            vert_flat_round: create!(device, "shaders/flat_round.vert.spv"),
            vert_round_2col: create!(device, "shaders/round_2col.vert.spv"),
            vert_shaded_round: create!(device, "shaders/shaded_round.vert.spv"),
            vert_shaded_square: create!(device, "shaders/shaded_square.vert.spv"),
            vert_image: create!(device, "shaders/image.vert.spv"),
            vert_glyph: create!(device, "shaders/glyph.vert.spv"),

            frag_flat_round: create!(device, "shaders/flat_round.frag.spv"),
            frag_round_2col: create!(device, "shaders/round_2col.frag.spv"),
            frag_shaded_round: create!(device, "shaders/shaded_round.frag.spv"),
            frag_shaded_square: create!(device, "shaders/shaded_square.frag.spv"),
            frag_image: create!(device, "shaders/image.frag.spv"),
            frag_glyph: create!(device, "shaders/glyph.frag.spv"),
            frag_subpixel: create!(device, "shaders/subpixel.frag.spv"),
        }
    }
}
