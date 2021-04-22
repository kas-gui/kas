// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded flat pipeline

use super::common;
use crate::draw::{Rgb, ShaderManager};
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Vec2};
use std::mem::size_of;

/// Offset relative to the size of a pixel used by the fragment shader to
/// implement multi-sampling.
const OFFSET: f32 = 0.125;

// NOTE(opt): in theory we could reduce data transmission to the GPU by 1/3 by
// sending quads (two triangles) as instances in triangle-strip mode. The
// "frame" shape could support four-triangle strips. However, this would require
// many rpass.draw() commands or shaders unpacking vertices from instance data.

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex(Vec2, Rgb, f32, Vec2, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    fn new2(v: Vec2, col: Rgb, inner: f32, n: Vec2, p: Vec2) -> Self {
        Vertex(v, col, inner, n, p)
    }
}

pub type Window = common::Window<Vertex>;

/// A pipeline for rendering rounded shapes
pub struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
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
                        0 => Float2,
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
                    alpha_blend: wgpu::BlendState::REPLACE,
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

    /// Enqueue render commands
    pub fn render<'a>(
        &'a self,
        window: &'a Window,
        pass: usize,
        rpass: &mut wgpu::RenderPass<'a>,
        bg_common: &'a wgpu::BindGroup,
    ) {
        window.render(pass, rpass, &self.render_pipeline, bg_common);
    }
}

impl Window {
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

        let p1my = p1 - vy;
        let p1py = p1 + vy;
        let p2my = p2 - vy;
        let p2py = p2 + vy;

        let ma = Vertex::new2(p1my, col, 0.0, Vec2(0.0, na.1), p);
        let mb = Vertex::new2(p1py, col, 0.0, Vec2(0.0, nb.1), p);
        let aa = Vertex::new2(p1my - vx, col, 0.0, Vec2(na.0, na.1), p);
        let ab = Vertex::new2(p1py - vx, col, 0.0, Vec2(na.0, nb.1), p);
        let ba = Vertex::new2(p2my + vx, col, 0.0, Vec2(nb.0, na.1), p);
        let bb = Vertex::new2(p2py + vx, col, 0.0, Vec2(nb.0, nb.1), p);
        let na = Vertex::new2(p2my, col, 0.0, Vec2(0.0, na.1), p);
        let nb = Vertex::new2(p2py, col, 0.0, Vec2(0.0, nb.1), p);
        let m = Vertex::new2(p1, col, 0.0, n0, p);
        let n = Vertex::new2(p2, col, 0.0, n0, p);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            ab, m, mb,
            mb, m, n,
            mb, n, nb,
            nb, n, bb,
            bb, n, ba,
            ba, n, na,
            na, n, ma,
            ma, n, m,
            ma, m, aa,
            aa, m, ab,
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

        let aa = Vertex::new2(aa, col, inner, na, p);
        let ab = Vertex::new2(ab, col, inner, nab, p);
        let ba = Vertex::new2(ba, col, inner, nba, p);
        let bb = Vertex::new2(bb, col, inner, nb, p);
        let mid = Vertex::new2(mid, col, inner, n0, p);

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

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 16 points:
        let ab = Vertex::new2(ab, col, inner, nab, pab);
        let ba = Vertex::new2(ba, col, inner, nba, pba);
        let cd = Vertex::new2(cd, col, inner, n0, pab);
        let dc = Vertex::new2(dc, col, inner, n0, pba);

        let ac = Vertex(Vec2(aa.0, cc.1), col, inner, na0, paa);
        let ad = Vertex(Vec2(aa.0, dd.1), col, inner, na0, pab);
        let bc = Vertex(Vec2(bb.0, cc.1), col, inner, nb0, pba);
        let bd = Vertex(Vec2(bb.0, dd.1), col, inner, nb0, pbb);

        let ca = Vertex(Vec2(cc.0, aa.1), col, inner, n0a, paa);
        let cb = Vertex(Vec2(cc.0, bb.1), col, inner, n0b, pab);
        let da = Vertex(Vec2(dd.0, aa.1), col, inner, n0a, pba);
        let db = Vertex(Vec2(dd.0, bb.1), col, inner, n0b, pbb);

        let aa = Vertex::new2(aa, col, inner, na, paa);
        let bb = Vertex::new2(bb, col, inner, nb, pbb);
        let cc = Vertex::new2(cc, col, inner, n0, paa);
        let dd = Vertex::new2(dd, col, inner, n0, pbb);

        // TODO: the four sides are simple rectangles, hence could use simpler rendering

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            // top bar: ba - dc - cc - aa
            ba, dc, da,
            da, dc, cc,
            da, cc, ca,
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
}
