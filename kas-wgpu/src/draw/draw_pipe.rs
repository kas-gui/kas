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
    DrawWindow, ShaderManager, Vec2, TEX_FORMAT,
};
use kas::draw::{Colour, Draw, DrawRounded, DrawShaded, DrawShared, Region};
use kas::geom::{Coord, Rect, Size};

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

        let region = Rect {
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
            clip_regions: vec![region],
            shaded_square,
            shaded_round,
            flat_round,
            custom,
            glyph_brush,
        }
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

        // We use a separate render pass for each clipped region.
        for (pass, region) in window.clip_regions.iter().enumerate() {
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
                region.pos.0 as u32,
                region.pos.1 as u32,
                region.size.0,
                region.size.1,
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

impl<CW: CustomWindow> DrawWindow<CW> {
    /// Process window resize
    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        self.clip_regions[0].size = size;
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.shaded_square.resize(device, &mut encoder, size);
        self.shaded_round.resize(device, &mut encoder, size);
        self.custom.resize(device, &mut encoder, size);
        self.flat_round.resize(device, &mut encoder, size);
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

    fn add_clip_region(&mut self, region: Rect) -> Region {
        let pass = self.clip_regions.len();
        self.clip_regions.push(region);
        Region(pass)
    }

    #[inline]
    fn rect(&mut self, pass: Region, rect: Rect, col: Colour) {
        self.shaded_square.rect(pass.0, rect, col);
    }

    #[inline]
    fn frame(&mut self, pass: Region, outer: Rect, inner: Rect, col: Colour) {
        self.shaded_square.frame(pass.0, outer, inner, col);
    }
}

impl<CW: CustomWindow + 'static> DrawRounded for DrawWindow<CW> {
    #[inline]
    fn rounded_line(&mut self, pass: Region, p1: Coord, p2: Coord, radius: f32, col: Colour) {
        self.flat_round.line(pass.0, p1, p2, radius, col);
    }

    #[inline]
    fn circle(&mut self, pass: Region, rect: Rect, inner_radius: f32, col: Colour) {
        self.flat_round.circle(pass.0, rect, inner_radius, col);
    }

    #[inline]
    fn rounded_frame(
        &mut self,
        pass: Region,
        outer: Rect,
        inner: Rect,
        inner_radius: f32,
        col: Colour,
    ) {
        self.flat_round
            .rounded_frame(pass.0, outer, inner, inner_radius, col);
    }
}

impl<CW: CustomWindow + 'static> DrawShaded for DrawWindow<CW> {
    #[inline]
    fn shaded_square(&mut self, pass: Region, rect: Rect, norm: (f32, f32), col: Colour) {
        self.shaded_square
            .shaded_rect(pass.0, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_circle(&mut self, pass: Region, rect: Rect, norm: (f32, f32), col: Colour) {
        self.shaded_round
            .circle(pass.0, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_square_frame(
        &mut self,
        pass: Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.shaded_square
            .shaded_frame(pass.0, outer, inner, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_round_frame(
        &mut self,
        pass: Region,
        outer: Rect,
        inner: Rect,
        norm: (f32, f32),
        col: Colour,
    ) {
        self.shaded_round
            .shaded_frame(pass.0, outer, inner, Vec2::from(norm), col);
    }
}
