// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use wgpu::{include_spirv, ShaderModule};

/// Shader manager
pub struct ShaderManager {
    pub vert_2: ShaderModule,
    pub vert_3122: ShaderModule,
    pub vert_32: ShaderModule,
    pub vert_322: ShaderModule,
    pub vert_3222: ShaderModule,
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
        let vert_2 = create!(device, "shaders/scaled2.vert.spv");
        let vert_3122 = create!(device, "shaders/scaled3122.vert.spv");
        let vert_32 = create!(device, "shaders/scaled32.vert.spv");
        let vert_322 = create!(device, "shaders/scaled322.vert.spv");
        let vert_3222 = create!(device, "shaders/scaled3222.vert.spv");

        let frag_flat_round = create!(device, "shaders/flat_round.frag.spv");
        let frag_shaded_square = create!(device, "shaders/shaded_square.frag.spv");
        let frag_shaded_round = create!(device, "shaders/shaded_round.frag.spv");
        let frag_image = create!(device, "shaders/image.frag.spv");

        ShaderManager {
            vert_2,
            vert_3122,
            vert_32,
            vert_322,
            vert_3222,
            frag_flat_round,
            frag_shaded_square,
            frag_shaded_round,
            frag_image,
        }
    }
}
