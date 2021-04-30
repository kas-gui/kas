// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple pipeline for "square" shading

use super::common;
use crate::draw::{Rgba, ShaderManager};
use kas::draw::{Colour, Pass};
use kas::geom::{Quad, Vec2};
use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex(Vec2, Rgba, Vec2);
unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

pub type Window = common::Window<Vertex>;

/// A pipeline for rendering with flat and square-corner shading
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
            label: Some("SS pipeline_layout"),
            bind_group_layouts: &[bgl_common],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SS render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shaders.vert_2,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: Some(wgpu::Face::Back), // not required
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shaders.frag_shaded_square,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
    /// Add a rectangle to the buffer
    pub fn rect(&mut self, pass: Pass, rect: Quad, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);

        let col = col.into();
        let t = Vec2(0.0, 0.0);

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(aa, col, t), Vertex(ba, col, t), Vertex(ab, col, t),
            Vertex(ab, col, t), Vertex(ba, col, t), Vertex(bb, col, t),
        ]);
    }

    /// Add a rect to the buffer, defined by two outer corners, `aa` and `bb`.
    ///
    /// Bounds on input: `aa < cc` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_rect(&mut self, pass: Pass, rect: Quad, mut norm: Vec2, col: Colour) {
        let aa = rect.a;
        let bb = rect.b;

        if !aa.lt(bb) {
            // zero / negative size: nothing to draw
            return;
        }
        if !Vec2::splat(-1.0).le(norm) || !norm.le(Vec2::splat(1.0)) {
            norm = Vec2::splat(0.0);
        }

        let mid = (aa + bb) * 0.5;
        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);

        let col = col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            Vertex(ba, col, tt.0), Vertex(mid, col, tt.1), Vertex(aa, col, tt.0),
            Vertex(aa, col, tl.0), Vertex(mid, col, tl.1), Vertex(ab, col, tl.0),
            Vertex(ab, col, tb.0), Vertex(mid, col, tb.1), Vertex(bb, col, tb.0),
            Vertex(bb, col, tr.0), Vertex(mid, col, tr.1), Vertex(ba, col, tr.0),
        ]);
    }

    #[inline]
    pub fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Colour) {
        let norm = Vec2::splat(0.0);
        self.shaded_frame(pass, outer, inner, norm, col, col);
    }

    /// Add a frame to the buffer, defined by two outer corners, `aa` and `bb`,
    /// and two inner corners, `cc` and `dd` with colours `outer_col`, `inner_col`.
    ///
    /// Bounds on input: `aa < cc < dd < bb` and `-1 ≤ norm ≤ 1`.
    pub fn shaded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        mut norm: Vec2,
        outer_col: Colour,
        inner_col: Colour,
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

        let ab = Vec2(aa.0, bb.1);
        let ba = Vec2(bb.0, aa.1);
        let cd = Vec2(cc.0, dd.1);
        let dc = Vec2(dd.0, cc.1);

        let outer_col = outer_col.into();
        let inner_col = inner_col.into();
        let tt = (Vec2(0.0, -norm.1), Vec2(0.0, -norm.0));
        let tl = (Vec2(-norm.1, 0.0), Vec2(-norm.0, 0.0));
        let tb = (Vec2(0.0, norm.1), Vec2(0.0, norm.0));
        let tr = (Vec2(norm.1, 0.0), Vec2(norm.0, 0.0));

        #[rustfmt::skip]
        self.add_vertices(pass.pass(), &[
            // top bar: ba - dc - cc - aa
            Vertex(ba, outer_col, tt.0), Vertex(dc, inner_col, tt.1), Vertex(aa, outer_col, tt.0),
            Vertex(aa, outer_col, tt.0), Vertex(dc, inner_col, tt.1), Vertex(cc, inner_col, tt.1),
            // left bar: aa - cc - cd - ab
            Vertex(aa, outer_col, tl.0), Vertex(cc, inner_col, tl.1), Vertex(ab, outer_col, tl.0),
            Vertex(ab, outer_col, tl.0), Vertex(cc, inner_col, tl.1), Vertex(cd, inner_col, tl.1),
            // bottom bar: ab - cd - dd - bb
            Vertex(ab, outer_col, tb.0), Vertex(cd, inner_col, tb.1), Vertex(bb, outer_col, tb.0),
            Vertex(bb, outer_col, tb.0), Vertex(cd, inner_col, tb.1), Vertex(dd, inner_col, tb.1),
            // right bar: bb - dd - dc - ba
            Vertex(bb, outer_col, tr.0), Vertex(dd, inner_col, tr.1), Vertex(ba, outer_col, tr.0),
            Vertex(ba, outer_col, tr.0), Vertex(dd, inner_col, tr.1), Vertex(dc, inner_col, tr.1),
        ]);
    }
}
