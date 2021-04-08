// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded shading pipeline

use std::f32::consts::FRAC_PI_2;
use std::mem::size_of;
use wgpu::util::DeviceExt;

use crate::draw::{Rgb, ShaderManager};
use kas::cast::Cast;
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Size, Vec2, Vec3};

/// Offset relative to the size of a pixel used by the fragment shader to
/// implement multi-sampling.
const OFFSET: f32 = 0.125;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Rgb, Vec2, Vec2, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    fn new2(v: Vec2, d: f32, col: Rgb, n: Vec2, adjust: Vec2, p: Vec2) -> Self {
        let v = Vec3::from2(v, d);
        Vertex(v, col, n, adjust, p)
    }
}

/// A pipeline for rendering rounded shapes
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

/// Buffer used during render pass
///
/// This buffer must not be dropped before the render pass.
pub struct RenderBuffer<'a> {
    pipe: &'a wgpu::RenderPipeline,
    vertices: &'a mut Vec<Vertex>,
    bind_group: &'a wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

impl<'a> RenderBuffer<'a> {
    /// Do the render
    pub fn render(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        let count = self.vertices.len().cast();
        rpass.set_pipeline(self.pipe);
        rpass.set_bind_group(0, self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.buffer.slice(..));
        rpass.draw(0..count, 0..1);
    }
}

impl<'a> Drop for RenderBuffer<'a> {
    fn drop(&mut self) {
        self.vertices.clear();
    }
}

impl Pipeline {
    /// Construct
    pub fn new(device: &wgpu::Device, shaders: &ShaderManager) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SR bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None, // TODO
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SR pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SR render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_3222,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float3,
                        1 => Float3,
                        2 => Float2,
                        3 => Float2,
                        4 => Float2
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: Some(super::DEPTH_DESC),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.frag_shaded_round,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    alpha_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    color_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Pipeline {
            bind_group_layout,
            render_pipeline,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, size: Size, light_norm: [f32; 3]) -> Window {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SR scale_buf"),
            contents: bytemuck::cast_slice(&scale_factor),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let light_norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SR light_norm_buf"),
            contents: bytemuck::cast_slice(&light_norm),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SR bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &scale_buf,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &light_norm_buf,
                        offset: 0,
                        size: None,
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

    /// Construct a render buffer
    pub fn render_buf<'a>(
        &'a self,
        window: &'a mut Window,
        device: &wgpu::Device,
        pass: usize,
    ) -> Option<RenderBuffer<'a>> {
        if pass >= window.passes.len() || window.passes[pass].len() == 0 {
            return None;
        }

        let vertices = &mut window.passes[pass];
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SR render_buf"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        Some(RenderBuffer {
            pipe: &self.render_pipeline,
            vertices,
            bind_group: &window.bind_group,
            buffer,
        })
    }
}

impl Window {
    pub fn resize(&mut self, queue: &wgpu::Queue, size: Size) {
        let scale_factor = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        queue.write_buffer(&self.scale_buf, 0, bytemuck::cast_slice(&scale_factor));
    }

    /// Bounds on input: `0 ≤ inner_radius ≤ 1`.
    pub fn circle(&mut self, pass: Pass, rect: Quad, mut norm: Vec2, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let adjust = Vec2(FRAC_PI_2 * norm.0, norm.1 - norm.0);
        let col = col.into();

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let mid = (aa + bb) * 0.5;

        let n0 = Vec2::splat(0.0);
        let nbb = (bb - aa).sign();
        let naa = -nbb;
        let nab = Vec2(naa.0, nbb.1);
        let nba = Vec2(nbb.0, naa.1);

        // Since we take the mid-point, all offsets are uniform
        let p = nbb / (bb - mid) * OFFSET;
        let depth = pass.depth();

        let aa = Vertex::new2(aa, depth, col, naa, adjust, p);
        let ab = Vertex::new2(ab, depth, col, nab, adjust, p);
        let ba = Vertex::new2(ba, depth, col, nba, adjust, p);
        let bb = Vertex::new2(bb, depth, col, nbb, adjust, p);
        let mid = Vertex::new2(mid, depth, col, n0, adjust, p);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            aa, ba, mid,
            mid, ba, bb,
            bb, ab, mid,
            mid, ab, aa,
        ]);
    }

    /// Bounds on input: `aa < cc < dd < bb` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        mut norm: Vec2,
        col: Colour,
    ) {
        let aa = outer.a;
        let bb = outer.b;
        let mut cc = inner.a;
        let mut dd = inner.b;

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

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        let n0 = Vec2::splat(0.0);
        let nbb = (bb - aa).sign();
        let naa = -nbb;
        let nab = Vec2(naa.0, nbb.1);
        let nba = Vec2(nbb.0, naa.1);
        let na0 = Vec2(naa.0, 0.0);
        let nb0 = Vec2(nbb.0, 0.0);
        let n0a = Vec2(0.0, naa.1);
        let n0b = Vec2(0.0, nbb.1);

        let paa = naa / (aa - cc) * OFFSET;
        let pab = nab / (ab - cd) * OFFSET;
        let pba = nba / (ba - dc) * OFFSET;
        let pbb = nbb / (bb - dd) * OFFSET;
        let depth = pass.depth();

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 16 points:
        let ab = Vertex::new2(ab, depth, col, nab, adjust, pab);
        let ba = Vertex::new2(ba, depth, col, nba, adjust, pba);
        let cd = Vertex::new2(cd, depth, col, n0, adjust, pab);
        let dc = Vertex::new2(dc, depth, col, n0, adjust, pba);

        let ac = Vertex(Vec3(aa.0, cc.1, depth), col, na0, adjust, paa);
        let ad = Vertex(Vec3(aa.0, dd.1, depth), col, na0, adjust, pab);
        let bc = Vertex(Vec3(bb.0, cc.1, depth), col, nb0, adjust, pba);
        let bd = Vertex(Vec3(bb.0, dd.1, depth), col, nb0, adjust, pbb);

        let ca = Vertex(Vec3(cc.0, aa.1, depth), col, n0a, adjust, paa);
        let cb = Vertex(Vec3(cc.0, bb.1, depth), col, n0b, adjust, pab);
        let da = Vertex(Vec3(dd.0, aa.1, depth), col, n0a, adjust, pba);
        let db = Vertex(Vec3(dd.0, bb.1, depth), col, n0b, adjust, pbb);

        let aa = Vertex::new2(aa, depth, col, naa, adjust, paa);
        let bb = Vertex::new2(bb, depth, col, nbb, adjust, pbb);
        let cc = Vertex::new2(cc, depth, col, n0, adjust, paa);
        let dd = Vertex::new2(dd, depth, col, n0, adjust, pbb);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
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
