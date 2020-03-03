// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple pipeline for "square" shading

use std::f32;
use std::mem::size_of;

use crate::draw::{Rgb, ShaderManager};
use kas::draw::Colour;
use kas::geom::{Rect, Size, Vec2};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec2, Rgb, Vec2);

/// A pipeline for rendering with flat and square-corner shading
pub struct Pipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

/// Per-window state
pub struct Window {
    bind_group: wgpu::BindGroup,
    scale_buf: wgpu::Buffer,
    passes: Vec<Vec<Vertex>>,
}

impl Pipeline {
    /// Construct
    pub fn new(device: &wgpu::Device, shaders: &ShaderManager) -> Self {
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &shaders.vert_32,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &shaders.frag_shaded_square,
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

        Pipeline {
            bind_group_layout,
            render_pipeline,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, size: Size, light_norm: [f32; 3]) -> Window {
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
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

        Window {
            bind_group,
            scale_buf,
            passes: vec![],
        }
    }

    /// Render queued triangles and clear the queue
    pub fn render(
        &self,
        window: &mut Window,
        device: &wgpu::Device,
        pass: usize,
        rpass: &mut wgpu::RenderPass,
    ) {
        if pass >= window.passes.len() {
            return;
        }
        let v = &mut window.passes[pass];
        let buffer = device
            .create_buffer_mapped(v.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&v);
        let count = v.len() as u32;

        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &window.bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&buffer, 0)]);
        rpass.draw(0..count, 0..1);

        v.clear();
    }
}

impl Window {
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

    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: usize, rect: Rect, col: Colour) {
        let pos = Vec2::from(rect.pos);
        let size = Vec2::from(rect.size);

        let (aa, bb) = (pos, pos + size);
        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);

        let col = col.into();
        let t = Vec2(0.0, 0.0);

        #[rustfmt::skip]
        self.add_vertices(pass, &[
            Vertex(aa, col, t), Vertex(ba, col, t), Vertex(ab, col, t),
            Vertex(ab, col, t), Vertex(ba, col, t), Vertex(bb, col, t),
        ]);
    }

    /// Add a rect to the buffer, defined by two outer corners, `aa` and `bb`.
    ///
    /// Bounds on input: `aa < cc` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_rect(&mut self, pass: usize, rect: Rect, mut norm: Vec2, col: Colour) {
        let aa = Vec2::from(rect.pos);
        let bb = aa + Vec2::from(rect.size);

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let mid = (aa + bb) * 0.5;

        let col = col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass, &[
            Vertex(ba, col, tt.0), Vertex(mid, col, tt.1), Vertex(aa, col, tt.0),
            Vertex(aa, col, tl.0), Vertex(mid, col, tl.1), Vertex(ab, col, tl.0),
            Vertex(ab, col, tb.0), Vertex(mid, col, tb.1), Vertex(bb, col, tb.0),
            Vertex(bb, col, tr.0), Vertex(mid, col, tr.1), Vertex(ba, col, tr.0),
        ]);
    }

    #[inline]
    pub fn frame(&mut self, pass: usize, outer: Rect, inner: Rect, col: Colour) {
        let norm = Vec2::splat(0.0);
        self.shaded_frame(pass, outer, inner, norm, col);
    }

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colour `col`.
    ///
    /// Bounds on input: `aa < cc < dd < bb` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_frame(
        &mut self,
        pass: usize,
        outer: Rect,
        inner: Rect,
        mut norm: Vec2,
        col: Colour,
    ) {
        let aa = Vec2::from(outer.pos);
        let bb = aa + Vec2::from(outer.size);
        let mut cc = Vec2::from(inner.pos);
        let mut dd = cc + Vec2::from(inner.size);

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

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        let col = col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass, &[
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

    fn add_vertices(&mut self, pass: usize, slice: &[Vertex]) {
        if self.passes.len() <= pass {
            // We only need one more, but no harm in adding extra
            self.passes.resize(pass + 8, vec![]);
        }

        self.passes[pass].extend_from_slice(slice);
    }
}
