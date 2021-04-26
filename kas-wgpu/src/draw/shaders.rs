// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use wgpu::{include_spirv, ShaderModule};

/// Shader manager
pub struct ShaderManager {
    pub vert_tex_quad: ShaderModule,
    pub vert_122: ShaderModule,
    pub vert_2: ShaderModule,
    pub vert_222: ShaderModule,
    pub frag_flat_round: ShaderModule,
    pub frag_shaded_square: ShaderModule,
    pub frag_shaded_round: ShaderModule,
    pub frag_image: ShaderModule,
}

macro_rules! create {
    ($device:ident, $path:expr) => {{
        $device.create_shader_module(&include_spirv!($path))
    }};
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let vert_tex_quad = create!(device, "shaders/tex_quad.vert.spv");
        let vert_122 = create!(device, "shaders/scaled_122.vert.spv");
        let vert_2 = create!(device, "shaders/scaled_2.vert.spv");
        let vert_222 = create!(device, "shaders/scaled_222.vert.spv");

        let frag_flat_round = create!(device, "shaders/flat_round.frag.spv");
        let frag_shaded_square = create!(device, "shaders/shaded_square.frag.spv");
        let frag_shaded_round = create!(device, "shaders/shaded_round.frag.spv");
        let frag_image = create!(device, "shaders/image.frag.spv");

        ShaderManager {
            vert_tex_quad,
            vert_122,
            vert_2,
            vert_222,
            frag_flat_round,
            frag_shaded_square,
            frag_shaded_round,
            frag_image,
        }
    }
}
