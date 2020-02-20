// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shader management

use shaderc::ShaderKind::{Fragment, Vertex};
use shaderc::{Compiler, Error};
use wgpu::ShaderModule;

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

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Result<Self, Error> {
        let mut compiler = Compiler::new().unwrap();

        let fname = "shaders/scaled3122.vert";
        let source = include_str!("shaders/scaled3122.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let vert_3122 = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/scaled32.vert";
        let source = include_str!("shaders/scaled32.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let vert_32 = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/scaled322.vert";
        let source = include_str!("shaders/scaled322.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let vert_322 = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/scaled3222.vert";
        let source = include_str!("shaders/scaled3222.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let vert_3222 = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/flat_round.frag";
        let source = include_str!("shaders/flat_round.frag");
        let artifact = compiler.compile_into_spirv(source, Fragment, fname, "main", None)?;
        let frag_flat_round = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/shaded_square.frag";
        let source = include_str!("shaders/shaded_square.frag");
        let artifact = compiler.compile_into_spirv(source, Fragment, fname, "main", None)?;
        let frag_shaded_square = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/shaded_round.frag";
        let source = include_str!("shaders/shaded_round.frag");
        let artifact = compiler.compile_into_spirv(source, Fragment, fname, "main", None)?;
        let frag_shaded_round = device.create_shader_module(&artifact.as_binary());

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
