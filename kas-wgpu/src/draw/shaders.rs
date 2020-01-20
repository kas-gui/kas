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
    pub square_vertex: ShaderModule,
    pub square_fragment: ShaderModule,
    pub round_vertex: ShaderModule,
    pub round_fragment: ShaderModule,
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Result<Self, Error> {
        let mut compiler = Compiler::new().unwrap();

        let fname = "shaders/square.vert";
        let source = include_str!("shaders/square.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let square_vertex = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/square.frag";
        let source = include_str!("shaders/square.frag");
        let artifact = compiler.compile_into_spirv(source, Fragment, fname, "main", None)?;
        let square_fragment = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/round.vert";
        let source = include_str!("shaders/round.vert");
        let artifact = compiler.compile_into_spirv(source, Vertex, fname, "main", None)?;
        let round_vertex = device.create_shader_module(&artifact.as_binary());

        let fname = "shaders/round.frag";
        let source = include_str!("shaders/round.frag");
        let artifact = compiler.compile_into_spirv(source, Fragment, fname, "main", None)?;
        let round_fragment = device.create_shader_module(&artifact.as_binary());

        Ok(ShaderManager {
            square_vertex,
            square_fragment,
            round_vertex,
            round_fragment,
        })
    }
}
