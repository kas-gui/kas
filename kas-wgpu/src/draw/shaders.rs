// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use shaderc::ShaderKind::{Fragment, Vertex};
use shaderc::{Compiler, Error};
use wgpu::{ShaderModule, ShaderModuleSource};

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
    ($device:ident, $compiler:ident, $type:ident, $path:expr) => {{
        let fname = $path;
        let source = include_str!($path);
        let artifact = $compiler.compile_into_spirv(source, $type, fname, "main", None)?;
        let bin = ShaderModuleSource::SpirV(artifact.as_binary().into());
        $device.create_shader_module(bin)
    }};
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Result<Self, Error> {
        let mut compiler = Compiler::new().unwrap();

        let vert_3122 = compile!(device, compiler, Vertex, "shaders/scaled3122.vert");
        let vert_32 = compile!(device, compiler, Vertex, "shaders/scaled32.vert");
        let vert_322 = compile!(device, compiler, Vertex, "shaders/scaled322.vert");
        let vert_3222 = compile!(device, compiler, Vertex, "shaders/scaled3222.vert");

        let frag_flat_round = compile!(device, compiler, Fragment, "shaders/flat_round.frag");
        let frag_shaded_square = compile!(device, compiler, Fragment, "shaders/shaded_square.frag");
        let frag_shaded_round = compile!(device, compiler, Fragment, "shaders/shaded_round.frag");

        Ok(ShaderManager {
            vert_3122,
            vert_32,
            vert_322,
            vert_3222,
            frag_flat_round,
            frag_shaded_square,
            frag_shaded_round,
        })
    }
}
