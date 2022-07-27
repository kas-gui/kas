// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use wgpu::{include_spirv, include_wgsl, ShaderModule};

/// Shader manager
pub struct ShaderManager {
    pub vert_flat_round: ShaderModule,
    pub vert_round_2col: ShaderModule,
    pub vert_shaded_round: ShaderModule,
    pub vert_image: ShaderModule,
    pub vert_glyph: ShaderModule,
    pub frag_flat_round: ShaderModule,
    pub frag_round_2col: ShaderModule,
    pub frag_shaded_round: ShaderModule,
    pub frag_image: ShaderModule,
    pub frag_glyph: ShaderModule,
    pub shaded_square: ShaderModule,
}

macro_rules! create {
    ($device:ident, $path:expr) => {{
        $device.create_shader_module(include_spirv!($path))
    }};
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let vert_flat_round = create!(device, "shaders/flat_round.vert.spv");
        let vert_round_2col = create!(device, "shaders/round_2col.vert.spv");
        let vert_shaded_round = create!(device, "shaders/shaded_round.vert.spv");
        let vert_image = create!(device, "shaders/image.vert.spv");
        let vert_glyph = create!(device, "shaders/glyph.vert.spv");

        let frag_flat_round = create!(device, "shaders/flat_round.frag.spv");
        let frag_round_2col = create!(device, "shaders/round_2col.frag.spv");
        let frag_shaded_round = create!(device, "shaders/shaded_round.frag.spv");
        let frag_image = create!(device, "shaders/image.frag.spv");
        let frag_glyph = create!(device, "shaders/glyph.frag.spv");

        let shaded_square =
            device.create_shader_module(include_wgsl!("shaders/shaded_square.wgsl"));

        ShaderManager {
            vert_image,
            vert_glyph,
            vert_flat_round,
            vert_round_2col,
            vert_shaded_round,
            frag_flat_round,
            frag_round_2col,
            frag_shaded_round,
            frag_image,
            frag_glyph,
            shaded_square,
        }
    }
}
