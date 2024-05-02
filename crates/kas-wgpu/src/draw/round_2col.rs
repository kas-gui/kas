// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Rounded two-colour pipeline

use super::common;
use crate::draw::ShaderManager;
use kas::draw::{color::Rgba, PassId};
use kas::geom::{Quad, Vec2};
use std::mem::size_of;

// NOTE(opt): in theory we could reduce data transmission to the GPU by 1/3 by
// sending quads (two triangles) as instances in triangle-strip mode. The
// "frame" shape could support four-triangle strips. However, this would require
// many rpass.draw() commands or shaders unpacking vertices from instance data.

/// Vertex
///
/// -   `screen_pos: Vec2` — screen coordinate
/// -   `col1: Rgba`
/// -   `col2: Rgba`
/// -   `circle_pos: Vec2` — coordinate on virtual circle with radius 1 centred
///     on the origin
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex(Vec2, Rgba, Rgba, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

pub type Window = common::Window<Vertex>;

/// A pipeline for rendering rounded shapes
///
/// Does not use anti-aliasing since edges usually have low alpha (opacity).
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
            label: Some("R2C pipeline_layout"),
            bind_group_layouts: &[bgl_common],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("R2C render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_round_2col,
                entry_point: "main",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x2,
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
                module: &shaders.frag_round_2col,
                entry_point: "main",
                compilation_options: Default::default(),
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
    pub fn circle(&mut self, pass: PassId, rect: Quad, col1: Rgba, col2: Rgba) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) || (col1.a == 0.0 && col2.a == 0.0) {
            // zero / negative size or transparent: nothing to draw
            return;
        }

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let mid = (aa + bb) * 0.5;

        let n0 = Vec2::splat(0.0);
        let nb = Vec2::ONE; // = (bb - aa).sign();
        let na = -nb;
        let nab = Vec2(na.0, nb.1);
        let nba = Vec2(nb.0, na.1);

        let aa = Vertex(aa, col1, col2, na);
        let ab = Vertex(ab, col1, col2, nab);
        let ba = Vertex(ba, col1, col2, nba);
        let bb = Vertex(bb, col1, col2, nb);
        let mid = Vertex(mid, col1, col2, n0);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            ba, mid, aa,
            bb, mid, ba,
            ab, mid, bb,
            aa, mid, ab,
        ]);
    }

    /// Bounds on input: `aa < cc < dd < bb`.
    pub fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col1: Rgba, col2: Rgba) {
        let aa = outer.a;
        let bb = outer.b;
        let mut cc = inner.a;
        let mut dd = inner.b;

        if !aa.lt(bb) || (col1.a == 0.0 && col2.a == 0.0) {
            // zero / negative size or transparent: nothing to draw
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

        // We must add corners separately to ensure correct interpolation of dir
        // values, hence need 16 points:
        let ab = Vertex(ab, col1, col2, nab);
        let ba = Vertex(ba, col1, col2, nba);
        let cd = Vertex(cd, col1, col2, n0);
        let dc = Vertex(dc, col1, col2, n0);

        let ac = Vertex(Vec2(aa.0, cc.1), col1, col2, na0);
        let ad = Vertex(Vec2(aa.0, dd.1), col1, col2, na0);
        let bc = Vertex(Vec2(bb.0, cc.1), col1, col2, nb0);
        let bd = Vertex(Vec2(bb.0, dd.1), col1, col2, nb0);

        let ca = Vertex(Vec2(cc.0, aa.1), col1, col2, n0a);
        let cb = Vertex(Vec2(cc.0, bb.1), col1, col2, n0b);
        let da = Vertex(Vec2(dd.0, aa.1), col1, col2, n0a);
        let db = Vertex(Vec2(dd.0, bb.1), col1, col2, n0b);

        let aa = Vertex(aa, col1, col2, na);
        let bb = Vertex(bb, col1, col2, nb);
        let cc = Vertex(cc, col1, col2, n0);
        let dd = Vertex(dd, col1, col2, n0);

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
