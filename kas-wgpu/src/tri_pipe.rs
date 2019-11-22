// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple triangle pipeline

use std::f32;
use std::mem::size_of;

use kas::geom::Size;

use crate::colour::Colour;
use crate::vertex::{Rgb, Vec2};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec2, Rgb);

/// A pipeline for rendering triangles with flat and graded shading
pub struct TriPipe {
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    v: Vec<Vertex>,
}

impl TriPipe {
    /// Construct
    pub fn new(device: &wgpu::Device, size: Size) -> Self {
        let vs_bytes = read_glsl(
            include_str!("shaders/tri_buffer.vert"),
            glsl_to_spirv::ShaderType::Vertex,
        );
        let fs_bytes = read_glsl(
            include_str!("shaders/tri_buffer.frag"),
            glsl_to_spirv::ShaderType::Fragment,
        );

        let vs_module = device.create_shader_module(&vs_bytes);
        let fs_module = device.create_shader_module(&fs_bytes);

        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let uniform_buf = device
            .create_buffer_mapped(
                scale_factor.len(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&scale_factor);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false },
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &uniform_buf,
                    range: 0..(size_of::<Scale>() as u64),
                },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float3,
                        offset: size_of::<Vec2>() as u64,
                        shader_location: 1,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        TriPipe {
            bind_group,
            uniform_buf,
            render_pipeline,
            v: vec![],
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let uniform_buf = device
            .create_buffer_mapped(scale_factor.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&scale_factor);
        let byte_len = size_of::<Scale>() as u64;

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        encoder.copy_buffer_to_buffer(&uniform_buf, 0, &self.uniform_buf, 0, byte_len);
        encoder.finish()
    }

    /// Render queued triangles and clear the queue
    pub fn render(&mut self, device: &wgpu::Device, rpass: &mut wgpu::RenderPass) {
        let buffer = device
            .create_buffer_mapped(self.v.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&self.v);
        let count = self.v.len() as u32;

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&buffer, 0)]);
        rpass.draw(0..count, 0..1);

        self.v.clear();
    }

    /// Add a rectangle to the buffer defined by two corners, `aa` and `bb`
    /// with colour `col`.
    pub fn add_quad(&mut self, aa: Vec2, bb: Vec2, col: Colour) {
        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let col = col.into();

        #[rustfmt::skip]
        self.v.extend_from_slice(&[
            Vertex(aa, col), Vertex(ba, col), Vertex(ab, col),
            Vertex(ab, col), Vertex(ba, col), Vertex(bb, col),
        ]);
    }

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd`. Uses grey-scale shading from
    /// outer colour `co` to inner colour `ci`.
    pub fn add_frame(&mut self, aa: Vec2, bb: Vec2, cc: Vec2, dd: Vec2, co: Colour, ci: Colour) {
        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);
        let co = co.into();
        let ci = ci.into();

        #[rustfmt::skip]
        self.v.extend_from_slice(&[
            // top bar: ba - dc - cc - aa
            Vertex(ba, co), Vertex(dc, ci), Vertex(aa, co),
            Vertex(aa, co), Vertex(dc, ci), Vertex(cc, ci),
            // left bar: aa - cc - cd - ab
            Vertex(aa, co), Vertex(cc, ci), Vertex(ab, co),
            Vertex(ab, co), Vertex(cc, ci), Vertex(cd, ci),
            // bottom bar: ab - cd - dd - bb
            Vertex(ab, co), Vertex(cd, ci), Vertex(bb, co),
            Vertex(bb, co), Vertex(cd, ci), Vertex(dd, ci),
            // right bar: bb - dd - dc - ba
            Vertex(bb, co), Vertex(dd, ci), Vertex(ba, co),
            Vertex(ba, co), Vertex(dd, ci), Vertex(dc, ci),
        ]);
    }
}

pub fn read_glsl(code: &str, stage: glsl_to_spirv::ShaderType) -> Vec<u32> {
    wgpu::read_spirv(glsl_to_spirv::compile(&code, stage).unwrap()).unwrap()
}
