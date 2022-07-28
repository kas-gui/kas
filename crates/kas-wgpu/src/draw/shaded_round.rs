// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded shading pipeline

use super::common;
use crate::draw::ShaderManager;
use kas::draw::{color::Rgba, PassId};
use kas::geom::{Quad, Vec2};
use std::f32::consts::FRAC_PI_2;
use std::mem::size_of;

/// Offset relative to the size of a pixel used by the fragment shader to
/// implement 4x multi-sampling. The pattern is defined by the fragment shader.
const OFFSET: f32 = 0.5 * std::f32::consts::FRAC_1_SQRT_2;

/// Vertex
///
/// -   `screen_pos: Vec2` — screen coordinate
/// -   `colour: Rgba`
/// -   `dir: Vec2` — normalised direction of slope (from (-1, -1) to (1, 1))
/// -   `adjust: Vec2`
/// -   `pix_offset: Vec2` — offset for a pixel; used for AA
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex(Vec2, Rgba, Vec2, Vec2, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

impl Vertex {
    fn new2(v: Vec2, col: Rgba, n: Vec2, adjust: Vec2, p: Vec2) -> Self {
        Vertex(v, col, n, adjust, p)
    }
}

pub type Window = common::Window<Vertex>;

/// A pipeline for rendering rounded shapes
///
/// Uses 4x sampling for anti-aliasing.
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
            label: Some("SR pipeline_layout"),
            bind_group_layouts: &[bgl_common],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SR render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_shaded_round,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x4,
                        2 => Float32x2,
                        3 => Float32x2,
                        4 => Float32x2
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back), // not required
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.frag_shaded_round,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: super::RENDER_TEX_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
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
    pub fn circle(&mut self, pass: PassId, rect: Quad, mut norm: Vec2, col: Rgba) {
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

        let aa = Vertex::new2(aa, col, naa, adjust, p);
        let ab = Vertex::new2(ab, col, nab, adjust, p);
        let ba = Vertex::new2(ba, col, nba, adjust, p);
        let bb = Vertex::new2(bb, col, nbb, adjust, p);
        let mid = Vertex::new2(mid, col, n0, adjust, p);

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
        pass: PassId,
        outer: Quad,
        inner: Quad,
        mut norm: Vec2,
        col: Rgba,
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

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 16 points:
        let ab = Vertex::new2(ab, col, nab, adjust, pab);
        let ba = Vertex::new2(ba, col, nba, adjust, pba);
        let cd = Vertex::new2(cd, col, n0, adjust, pab);
        let dc = Vertex::new2(dc, col, n0, adjust, pba);

        let ac = Vertex(Vec2(aa.0, cc.1), col, na0, adjust, paa);
        let ad = Vertex(Vec2(aa.0, dd.1), col, na0, adjust, pab);
        let bc = Vertex(Vec2(bb.0, cc.1), col, nb0, adjust, pba);
        let bd = Vertex(Vec2(bb.0, dd.1), col, nb0, adjust, pbb);

        let ca = Vertex(Vec2(cc.0, aa.1), col, n0a, adjust, paa);
        let cb = Vertex(Vec2(cc.0, bb.1), col, n0b, adjust, pab);
        let da = Vertex(Vec2(dd.0, aa.1), col, n0a, adjust, pba);
        let db = Vertex(Vec2(dd.0, bb.1), col, n0b, adjust, pbb);

        let aa = Vertex::new2(aa, col, naa, adjust, paa);
        let bb = Vertex::new2(bb, col, nbb, adjust, pbb);
        let cc = Vertex::new2(cc, col, n0, adjust, paa);
        let dd = Vertex::new2(dd, col, n0, adjust, pbb);

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
}
