// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use wgpu::{include_spirv, ShaderModule};

/// Shader manager
///
/// For now, we embed the shader source into the binary and compile on start.
/// Not really optimal (we could embed SPIR-V directly or load shaders from
/// external resources), but simple to set up and use.
pub struct ShaderManager {
    pub vert_3122: ShaderModule,
    pub vert_32: ShaderModule,
    pub vert_322: ShaderModule,
    pub vert_3222: ShaderModule,
    pub frag_flat_round: ShaderModule,
    pub frag_shaded_square: ShaderModule,
    pub frag_shaded_round: ShaderModule,
}

macro_rules! compile {
    ($device:ident, $path:expr) => {{
        $device.create_shader_module(&include_spirv!($path))
    }};
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let vert_3122 = compile!(device, "shaders/scaled3122.vert.spv");
        let vert_32 = compile!(device, "shaders/scaled32.vert.spv");
        let vert_322 = compile!(device, "shaders/scaled322.vert.spv");
        let vert_3222 = compile!(device, "shaders/scaled3222.vert.spv");

        let frag_flat_round = compile!(device, "shaders/flat_round.frag.spv");
        let frag_shaded_square = compile!(device, "shaders/shaded_square.frag.spv");
        let frag_shaded_round = compile!(device, "shaders/shaded_round.frag.spv");

        ShaderManager {
            vert_3122,
            vert_32,
            vert_322,
            vert_3222,
            frag_flat_round,
            frag_shaded_square,
            frag_shaded_round,
        }
    }
}
