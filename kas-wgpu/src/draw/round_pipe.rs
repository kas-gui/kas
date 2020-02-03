// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded shading pipeline

use std::f32::consts::FRAC_PI_2;
use std::mem::size_of;

use crate::draw::{Colour, Quad, Vec2};
use kas::geom::Size;

use super::Rgb;
use crate::shared::SharedState;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec2, Rgb, Vec2, Vec2);

/// A pipeline for rendering rounded shapes
pub struct RoundPipe {
    bind_group: wgpu::BindGroup,
    scale_buf: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    passes: Vec<Vec<Vertex>>,
}

impl RoundPipe {
    /// Construct
    pub fn new<T>(shared: &SharedState<T>, size: Size, light_norm: [f32; 3]) -> Self {
        let device = &shared.device;

        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, 2.0 / size.1 as f32];
        let scale_buf = device
            .create_buffer_mapped(
                scale_factor.len(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&scale_factor);

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
                        range: 0..(size_of::<[f32; 3]>() as u64),
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
                module: &shared.shaders.round_vertex,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &shared.shaders.round_fragment,
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
                    wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float2,
                        offset: (2 * size_of::<Vec2>() + size_of::<Rgb>()) as u64,
                        shader_location: 3,
                    },
                ],
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        RoundPipe {
            bind_group,
            scale_buf,
            render_pipeline,
            passes: vec![],
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
    pub fn render(&mut self, device: &wgpu::Device, pass: usize, rpass: &mut wgpu::RenderPass) {
        if pass >= self.passes.len() {
            return;
        }
        let v = &mut self.passes[pass];
        let buffer = device
            .create_buffer_mapped(v.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&v);
        let count = v.len() as u32;

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&buffer, 0)]);
        rpass.draw(0..count, 0..1);

        v.clear();
    }

    /// Bounds on input: `aa < cc < dd < bb` and `-1 ≤ norm ≤ 1`.
    pub fn add_frame(
        &mut self,
        pass: usize,
        outer: Quad,
        inner: Quad,
        mut norm: Vec2,
        col: Colour,
    ) {
        let (aa, bb) = (outer.0, outer.1);
        let (mut cc, mut dd) = (inner.0, inner.1);

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !aa.le(cc) || !cc.le(bb) {
            cc = aa;
        }
        if !aa.le(dd) || !dd.le(bb) {
            dd = bb;
        }
        if !cc.le(dd) {
            dd = cc;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let adjust = Vec2(FRAC_PI_2 * norm.0, norm.1 - norm.0);
        let col = col.into();

        let n0 = Vec2::splat(0.0);
        let nbb = (bb - aa).sign();
        let naa = -nbb;
        let nab = Vec2(naa.0, nbb.1);
        let nba = Vec2(nbb.0, naa.1);
        let na0 = Vec2(naa.0, 0.0);
        let nb0 = Vec2(nbb.0, 0.0);
        let n0a = Vec2(0.0, naa.1);
        let n0b = Vec2(0.0, nbb.1);

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 12 points:
        let ab = Vertex(Vec2(aa.0, bb.1), col, nab, adjust);
        let ba = Vertex(Vec2(bb.0, aa.1), col, nba, adjust);
        let cd = Vertex(Vec2(cc.0, dd.1), col, n0, adjust);
        let dc = Vertex(Vec2(dd.0, cc.1), col, n0, adjust);

        let ac = Vertex(Vec2(aa.0, cc.1), col, na0, adjust);
        let ad = Vertex(Vec2(aa.0, dd.1), col, na0, adjust);
        let bc = Vertex(Vec2(bb.0, cc.1), col, nb0, adjust);
        let bd = Vertex(Vec2(bb.0, dd.1), col, nb0, adjust);

        let ca = Vertex(Vec2(cc.0, aa.1), col, n0a, adjust);
        let cb = Vertex(Vec2(cc.0, bb.1), col, n0b, adjust);
        let da = Vertex(Vec2(dd.0, aa.1), col, n0a, adjust);
        let db = Vertex(Vec2(dd.0, bb.1), col, n0b, adjust);

        let aa = Vertex(aa, col, naa, adjust);
        let bb = Vertex(bb, col, nbb, adjust);
        let cc = Vertex(cc, col, n0, adjust);
        let dd = Vertex(dd, col, n0, adjust);

        #[rustfmt::skip]
        self.add_vertices(pass, &[
            // top bar: ba - dc - cc - aa
            ba, dc, da,
            da, dc, ca,
            dc, cc, ca,
            ca, cc, aa,
            // left bar: aa - cc - cd - ab
            aa, cc, ac,
            ac, cc, cd,
            ac, cd, ad,
            ad, cd, ab,
            // bottom bar: ab - cd - dd - bb
            ab, cd, cb,
            cb, cd, dd,
            cb, dd, db,
            db, dd, bb,
            // right bar: bb - dd - dc - ba
            bb, dd, bd,
            bd, dd, dc,
            bd, dc, bc,
            bc, dc, ba,
        ]);
    }

    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }

        self.passes[pass].extend_from_slice(slice);
    }
}
