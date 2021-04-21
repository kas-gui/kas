// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded flat pipeline

use std::mem::size_of;
use std::num::NonZeroU64;
use std::ops::Range;

use crate::draw::{Rgb, ShaderManager};
use kas::cast::Conv;
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Vec2, Vec3};

/// Offset relative to the size of a pixel used by the fragment shader to
/// implement multi-sampling.
const OFFSET: f32 = 0.125;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex(Vec3, Rgb, f32, Vec2, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    fn new2(v: Vec2, d: f32, col: Rgb, inner: f32, n: Vec2, p: Vec2) -> Self {
        let v = Vec3::from2(v, d);
        Vertex(v, col, inner, n, p)
    }
}

/// A pipeline for rendering rounded shapes
pub struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
}

#[derive(Clone, Debug, Default)]
struct PassData {
    vertices: Vec<Vertex>,
    count: u32,
    data_range: Range<u64>,
}

/// Per-window state
pub struct Window {
    passes: Vec<PassData>,
    buffer: Option<wgpu::Buffer>,
    buffer_size: u64,
}

impl Pipeline {
    /// Construct
    pub fn new(
        device: &wgpu::Device,
        shaders: &ShaderManager,
        bgl_common: &wgpu::BindGroupLayout,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FR pipeline_layout"),
            bind_group_layouts: &[bgl_common],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FR render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_3122,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float3,
                        1 => Float3,
                        2 => Float,
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
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.frag_flat_round,
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

        Pipeline { render_pipeline }
    }

    /// Construct per-window state
    pub fn new_window(&self) -> Window {
        Window {
            passes: vec![],
            buffer: None,
            buffer_size: 0,
        }
    }

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        if let Some(buffer) = window.buffer.as_ref() {
            if let Some(pass) = window.passes.get(pass) {
                rpass.set_pipeline(&self.render_pipeline);
                rpass.set_bind_group(0, bg_common, &[]);
                rpass.set_vertex_buffer(0, buffer.slice(pass.data_range.clone()));
                rpass.draw(0..pass.count, 0..1);
            }
        }
    }
}

impl Window {
    /// Prepare vertex buffers
    pub fn write_buffers(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let req_len = self
            .passes
            .iter()
            .map(|pd| u64::conv(pd.vertices.len() * size_of::<Vertex>()))
            .sum();
        let byte_len = match NonZeroU64::new(req_len) {
            Some(nz) => nz,
            None => return,
        };

        if req_len <= self.buffer_size {
            let buffer = self.buffer.as_ref().unwrap();
            let mut slice = staging_belt.write_buffer(encoder, buffer, 0, byte_len, device);
            copy_to_slice(&mut self.passes, &mut slice);
        } else {
            // Size must be a multiple of alignment
            let mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
            let buffer_size = (byte_len.get() + mask) & !mask;
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("flat_round vertex buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: true,
            });

            let mut slice = buffer.slice(..byte_len.get()).get_mapped_range_mut();
            copy_to_slice(&mut self.passes, &mut slice);
            drop(slice);

            buffer.unmap();
            self.buffer = Some(buffer);
            self.buffer_size = buffer_size;
        }

        fn copy_to_slice(passes: &mut [PassData], slice: &mut [u8]) {
            let mut byte_offset = 0;
            for pass in passes.iter_mut() {
                let len = u32::conv(pass.vertices.len());
                let byte_len = u64::from(len) * u64::conv(size_of::<Vertex>());
                let byte_end = byte_offset + byte_len;

                slice[usize::conv(byte_offset)..usize::conv(byte_end)]
                    .copy_from_slice(bytemuck::cast_slice(&pass.vertices));

                pass.vertices.clear();
                pass.count = len;
                pass.data_range = byte_offset..byte_end;
                byte_offset = byte_end;
            }
        }
    }

    pub fn line(&mut self, pass: Pass, p1: Vec2, p2: Vec2, radius: f32, col: Colour) {
        if p1 == p2 {
            let a = p1 - radius;
            let b = p2 + radius;
            self.circle(pass, Quad { a, b }, radius, col);
            return;
        }

        let col = col.into();

        let vx = p2 - p1;
        let vx = vx * radius / (vx.0 * vx.0 + vx.1 * vx.1).sqrt();
        let vy = Vec2(-vx.1, vx.0);

        let n0 = Vec2::splat(0.0);
        let nb = (vx + vy).sign();
        let na = -nb;

        // Since we take the mid-point, all offsets are uniform
        let p = Vec2::splat(OFFSET / radius);

        let depth = pass.depth();
        let p1my = p1 - vy;
        let p1py = p1 + vy;
        let p2my = p2 - vy;
        let p2py = p2 + vy;

        let ma1 = Vertex::new2(p1my, depth, col, 0.0, Vec2(0.0, na.1), p);
        let mb1 = Vertex::new2(p1py, depth, col, 0.0, Vec2(0.0, nb.1), p);
        let aa1 = Vertex::new2(p1my - vx, depth, col, 0.0, Vec2(na.0, na.1), p);
        let ab1 = Vertex::new2(p1py - vx, depth, col, 0.0, Vec2(na.0, nb.1), p);
        let ma2 = Vertex::new2(p2my, depth, col, 0.0, Vec2(0.0, na.1), p);
        let mb2 = Vertex::new2(p2py, depth, col, 0.0, Vec2(0.0, nb.1), p);
        let ba2 = Vertex::new2(p2my + vx, depth, col, 0.0, Vec2(nb.0, na.1), p);
        let bb2 = Vertex::new2(p2py + vx, depth, col, 0.0, Vec2(nb.0, nb.1), p);
        let p1 = Vertex::new2(p1, depth, col, 0.0, n0, p);
        let p2 = Vertex::new2(p2, depth, col, 0.0, n0, p);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            ab1, p1, mb1,
            aa1, p1, ab1,
            ma1, p1, aa1,
            mb1, p1, mb2,
            mb2, p1, p2,
            mb2, p2, bb2,
            bb2, p2, ba2,
            ba2, p2, ma2,
            ma2, p2, p1,
            p1, ma1, ma2,
        ]);
    }

    /// Bounds on input: `0 ≤ inner_radius ≤ 1`.
    pub fn circle(&mut self, pass: Pass, rect: Quad, inner_radius: f32, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let inner = inner_radius.max(0.0).min(1.0);

        let col = col.into();

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let mid = (aa + bb) * 0.5;

        let n0 = Vec2::splat(0.0);
        let nb = (bb - aa).sign();
        let na = -nb;
        let nab = Vec2(na.0, nb.1);
        let nba = Vec2(nb.0, na.1);

        // Since we take the mid-point, all offsets are uniform
        let p = nb / (bb - mid) * OFFSET;
        let depth = pass.depth();

        let aa = Vertex::new2(aa, depth, col, inner, na, p);
        let ab = Vertex::new2(ab, depth, col, inner, nab, p);
        let ba = Vertex::new2(ba, depth, col, inner, nba, p);
        let bb = Vertex::new2(bb, depth, col, inner, nb, p);
        let mid = Vertex::new2(mid, depth, col, inner, n0, p);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            ba, mid, aa,
            bb, mid, ba,
            ab, mid, bb,
            aa, mid, ab,
        ]);
    }

    /// Bounds on input: `aa < cc < dd < bb`, `0 ≤ inner_radius ≤ 1`.
    pub fn rounded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        inner_radius: f32,
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

        let inner = inner_radius.max(0.0).min(1.0);

        let col = col.into();

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        let n0 = Vec2::splat(0.0);
        let nb = (bb - aa).sign();
        let na = -nb;
        let nab = Vec2(na.0, nb.1);
        let nba = Vec2(nb.0, na.1);
        let na0 = Vec2(na.0, 0.0);
        let nb0 = Vec2(nb.0, 0.0);
        let n0a = Vec2(0.0, na.1);
        let n0b = Vec2(0.0, nb.1);

        let paa = na / (aa - cc) * OFFSET;
        let pab = nab / (ab - cd) * OFFSET;
        let pba = nba / (ba - dc) * OFFSET;
        let pbb = nb / (bb - dd) * OFFSET;
        let depth = pass.depth();

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 16 points:
        let ab = Vertex::new2(ab, depth, col, inner, nab, pab);
        let ba = Vertex::new2(ba, depth, col, inner, nba, pba);
        let cd = Vertex::new2(cd, depth, col, inner, n0, pab);
        let dc = Vertex::new2(dc, depth, col, inner, n0, pba);

        let ac = Vertex(Vec3(aa.0, cc.1, depth), col, inner, na0, paa);
        let ad = Vertex(Vec3(aa.0, dd.1, depth), col, inner, na0, pab);
        let bc = Vertex(Vec3(bb.0, cc.1, depth), col, inner, nb0, pba);
        let bd = Vertex(Vec3(bb.0, dd.1, depth), col, inner, nb0, pbb);

        let ca = Vertex(Vec3(cc.0, aa.1, depth), col, inner, n0a, paa);
        let cb = Vertex(Vec3(cc.0, bb.1, depth), col, inner, n0b, pab);
        let da = Vertex(Vec3(dd.0, aa.1, depth), col, inner, n0a, pba);
        let db = Vertex(Vec3(dd.0, bb.1, depth), col, inner, n0b, pbb);

        let aa = Vertex::new2(aa, depth, col, inner, na, paa);
        let bb = Vertex::new2(bb, depth, col, inner, nb, pbb);
        let cc = Vertex::new2(cc, depth, col, inner, n0, paa);
        let dd = Vertex::new2(dd, depth, col, inner, n0, pbb);

        // TODO: the four sides are simple rectangles, hence could use simpler rendering

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
            self.passes.resize(pass + 8, Default::default());
        }

        self.passes[pass].vertices.extend_from_slice(slice);
    }
}
