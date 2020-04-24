// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`

use std::any::Any;
use std::f32::consts::FRAC_PI_2;
use wgpu_glyph::GlyphBrushBuilder;

use super::{
    flat_round, shaded_round, shaded_square, CustomPipe, CustomPipeBuilder, CustomWindow, DrawPipe,
    DrawWindow, ShaderManager, TEX_FORMAT,
};
use kas::draw::{Colour, Draw, DrawRounded, DrawShaded, DrawShared, Pass};
use kas::geom::{Coord, Quad, Rect, Size, Vec2};

impl<C: CustomPipe> DrawPipe<C> {
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        mut custom: CB,
        device: &wgpu::Device,
        shaders: &ShaderManager,
    ) -> Self {
        let shaded_square = shaded_square::Pipeline::new(device, shaders);
        let shaded_round = shaded_round::Pipeline::new(device, shaders);
        let flat_round = flat_round::Pipeline::new(device, shaders);
        let custom = custom.build(&device, TEX_FORMAT);

        DrawPipe {
            fonts: vec![],
            shaded_square,
            shaded_round,
            flat_round,
            custom,
        }
    }

    /// Construct per-window state
    // TODO: device should be &, not &mut (but for glyph_brush)
    pub fn new_window(&self, device: &mut wgpu::Device, size: Size) -> DrawWindow<C::Window> {
        // Light dir: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
        // normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
        // (from UP, clockwise), both in radians.
        let dir: (f32, f32) = (0.3, 0.4);
        assert!(dir.0 >= 0.0);
        assert!(dir.0 < FRAC_PI_2);
        let a = (dir.0.sin(), dir.0.cos());
        // We normalise intensity:
        let f = a.0 / a.1;
        let norm = [dir.1.sin() * f, -dir.1.cos() * f, 1.0];

        let rect = Rect {
            pos: Coord::ZERO,
            size,
        };

        let shaded_square = self.shaded_square.new_window(device, size, norm);
        let shaded_round = self.shaded_round.new_window(device, size, norm);
        let flat_round = self.flat_round.new_window(device, size);
        let custom = self.custom.new_window(device, size);

        let glyph_brush =
            GlyphBrushBuilder::using_fonts(self.fonts.clone()).build(device, TEX_FORMAT);

        DrawWindow {
            clip_regions: vec![rect],
            shaded_square,
            shaded_round,
            flat_round,
            custom,
            glyph_brush,
        }
    }

    /// Process window resize
    pub fn resize(
        &self,
        window: &mut DrawWindow<C::Window>,
        device: &wgpu::Device,
        size: Size,
    ) -> wgpu::CommandBuffer {
        window.clip_regions[0].size = size;
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        window.shaded_square.resize(device, &mut encoder, size);
        window.shaded_round.resize(device, &mut encoder, size);
        self.custom
            .resize(&mut window.custom, device, &mut encoder, size);
        window.flat_round.resize(device, &mut encoder, size);
        encoder.finish()
    }

    /// Render batched draw instructions via `rpass`
    pub fn render(
        &self,
        window: &mut DrawWindow<C::Window>,
        device: &mut wgpu::Device,
        frame_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
    ) -> wgpu::CommandBuffer {
        let desc = wgpu::CommandEncoderDescriptor { todo: 0 };
        let mut encoder = device.create_command_encoder(&desc);
        let mut load_op = wgpu::LoadOp::Clear;

        self.custom.update(&mut window.custom, device, &mut encoder);

        // We use a separate render pass for each clipped region.
        for (pass, rect) in window.clip_regions.iter().enumerate() {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: frame_view,
                    resolve_target: None,
                    load_op: load_op,
                    store_op: wgpu::StoreOp::Store,
                    clear_color,
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_scissor_rect(
                rect.pos.0 as u32,
                rect.pos.1 as u32,
                rect.size.0,
                rect.size.1,
            );

            self.shaded_square
                .render(&mut window.shaded_square, device, pass, &mut rpass);
            self.shaded_round
                .render(&mut window.shaded_round, device, pass, &mut rpass);
            self.flat_round
                .render(&mut window.flat_round, device, pass, &mut rpass);
            self.custom
                .render(&mut window.custom, device, pass, &mut rpass);
            drop(rpass);

            load_op = wgpu::LoadOp::Load;
        }

        // Fonts use their own render pass(es).
        let size = window.clip_regions[0].size;
        window
            .glyph_brush
            .draw_queued(device, &mut encoder, frame_view, size.0, size.1)
            .expect("glyph_brush.draw_queued");

        // Keep only first clip region (which is the entire window)
        window.clip_regions.truncate(1);

        encoder.finish()
    }
}

impl<C: CustomPipe> DrawShared for DrawPipe<C> {
    type Draw = DrawWindow<C::Window>;
}

impl<CW: CustomWindow + 'static> Draw for DrawWindow<CW> {
    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn add_clip_region(&mut self, rect: Rect, depth: f32) -> Pass {
        let pass = self.clip_regions.len();
        self.clip_regions.push(rect);
        Pass::new_pass_with_depth(pass as u32, depth)
    }

    #[inline]
    fn rect(&mut self, pass: Pass, rect: Quad, col: Colour) {
        self.shaded_square.rect(pass, rect, col);
    }

    #[inline]
    fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Colour) {
        self.shaded_square.frame(pass, outer, inner, col);
    }
}

impl<CW: CustomWindow + 'static> DrawRounded for DrawWindow<CW> {
    #[inline]
    fn rounded_line(&mut self, pass: Pass, p1: Vec2, p2: Vec2, radius: f32, col: Colour) {
        self.flat_round.line(pass, p1, p2, radius, col);
    }

    #[inline]
    fn circle(&mut self, pass: Pass, rect: Quad, inner_radius: f32, col: Colour) {
        self.flat_round.circle(pass, rect, inner_radius, col);
    }

    #[inline]
    fn rounded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        inner_radius: f32,
        col: Colour,
    ) {
        self.flat_round
            .rounded_frame(pass, outer, inner, inner_radius, col);
    }
}

impl<CW: CustomWindow + 'static> DrawShaded for DrawWindow<CW> {
    #[inline]
    fn shaded_square(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Colour) {
        self.shaded_square
            .shaded_rect(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_circle(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Colour) {
        self.shaded_round.circle(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_square_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.shaded_square
            .shaded_frame(pass, outer, inner, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_round_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.shaded_round
            .shaded_frame(pass, outer, inner, Vec2::from(norm), col);
    }
}
