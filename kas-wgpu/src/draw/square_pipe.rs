// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple pipeline for "square" shading

use std::f32;
use std::mem::size_of;

use kas::geom::Size;

use crate::colour::Colour;
use crate::vertex::{Rgb, Vec2};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec2, Rgb, Vec2);

/// A pipeline for rendering with flat and square-corner shading
pub struct SquarePipe {
    bind_group: wgpu::BindGroup,
    scale_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    v: Vec<Vertex>,
}

impl SquarePipe {
    /// Construct
    pub fn new(device: &wgpu::Device, size: Size) -> Self {
        let vs_bytes = super::read_glsl(
            include_str!("shaders/tri_buffer.vert"),
            glsl_to_spirv::ShaderType::Vertex,
        );
        let fs_bytes = super::read_glsl(
            include_str!("shaders/tri_buffer.frag"),
            glsl_to_spirv::ShaderType::Fragment,
        );

        let vs_module = device.create_shader_module(&vs_bytes);
        let fs_module = device.create_shader_module(&fs_bytes);

        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let scale_buf = device
            .create_buffer_mapped(
                scale_factor.len(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&scale_factor);

        type Norm = [f32; 3];
        let light_norm: Norm = [0.2588, -0.5, 0.8365];
        let light_norm_buf = device
            .create_buffer_mapped(
                light_norm.len(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&light_norm);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
                wgpu::BindGroupLayoutBinding {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &scale_buf,
                        range: 0..(size_of::<Scale>() as u64),
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &light_norm_buf,
                        range: 0..(size_of::<Norm>() as u64),
                    },
                },
            ],
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
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: (size_of::<Vec2>() + size_of::<Rgb>()) as u64,
                        shader_location: 2,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        SquarePipe {
            bind_group,
            scale_buf,
            render_pipeline,
            v: vec![],
        }
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        size: Size,
    ) {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let scale_buf = device
            .create_buffer_mapped(scale_factor.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&scale_factor);
        let byte_len = size_of::<Scale>() as u64;

        encoder.copy_buffer_to_buffer(&scale_buf, 0, &self.scale_buf, 0, byte_len);
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
        let t = Vec2(0.0, 0.0);

        #[rustfmt::skip]
        self.v.extend_from_slice(&[
            Vertex(aa, col, t), Vertex(ba, col, t), Vertex(ab, col, t),
            Vertex(ab, col, t), Vertex(ba, col, t), Vertex(bb, col, t),
        ]);
    }

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colour `col`.
    ///
    /// The frame is shaded according to normals `norm = (outer, inner)`. A
    /// normal of 0 implies a surface parallel to the screen, and 1
    /// perpendicular to the screen. Can be positive (sunk frame) or negative
    /// (raised frame).
    pub fn add_frame(
        &mut self,
        aa: Vec2,
        bb: Vec2,
        cc: Vec2,
        dd: Vec2,
        norm: (f32, f32),
        col: Colour,
    ) {
        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        let col = col.into();
        let tt = (Vec2(0.0, norm.0), Vec2(0.0, norm.1));
        let tl = (Vec2(norm.0, 0.0), Vec2(norm.1, 0.0));
        let tb = (Vec2(0.0, -norm.0), Vec2(0.0, -norm.1));
        let tr = (Vec2(-norm.0, 0.0), Vec2(-norm.1, 0.0));

        #[rustfmt::skip]
        self.v.extend_from_slice(&[
            // top bar: ba - dc - cc - aa
            Vertex(ba, col, tt.0), Vertex(dc, col, tt.1), Vertex(aa, col, tt.0),
            Vertex(aa, col, tt.0), Vertex(dc, col, tt.1), Vertex(cc, col, tt.1),
            // left bar: aa - cc - cd - ab
            Vertex(aa, col, tl.0), Vertex(cc, col, tl.1), Vertex(ab, col, tl.0),
            Vertex(ab, col, tl.0), Vertex(cc, col, tl.1), Vertex(cd, col, tl.1),
            // bottom bar: ab - cd - dd - bb
            Vertex(ab, col, tb.0), Vertex(cd, col, tb.1), Vertex(bb, col, tb.0),
            Vertex(bb, col, tb.0), Vertex(cd, col, tb.1), Vertex(dd, col, tb.1),
            // right bar: bb - dd - dc - ba
            Vertex(bb, col, tr.0), Vertex(dd, col, tr.1), Vertex(ba, col, tr.0),
            Vertex(ba, col, tr.0), Vertex(dd, col, tr.1), Vertex(dc, col, tr.1),
        ]);
    }
}
