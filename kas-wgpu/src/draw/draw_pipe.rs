// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`

use std::any::Any;
use std::f32::consts::FRAC_PI_2;
use wgpu::TextureView;
use wgpu_glyph::{ab_glyph::FontRef, GlyphBrushBuilder};

use super::{
    flat_round, shaded_round, shaded_square, CustomPipe, CustomPipeBuilder, CustomWindow, DrawPipe,
    DrawWindow, ShaderManager, TEX_FORMAT,
};
use kas::cast::Cast;
use kas::draw::{Colour, Draw, DrawRounded, DrawShaded, DrawShared, Pass};
use kas::geom::{Coord, Quad, Rect, Size, Vec2};

fn make_depth_texture(device: &wgpu::Device, size: Size) -> Option<TextureView> {
    // NOTE: initially the DrawWindow is created with Size::ZERO to calculate
    // initial window size. Wgpu does not support creation of zero-sized
    // textures, so as a special case we return None here:
    if size.0 * size.1 == 0 {
        return None;
    }

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("window depth"),
        size: wgpu::Extent3d {
            width: size.0.cast(),
            height: size.1.cast(),
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: super::DEPTH_FORMAT,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
    });
    Some(tex.create_view(&Default::default()))
}

impl<C: CustomPipe> DrawPipe<C> {
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        mut custom: CB,
        device: &wgpu::Device,
        shaders: &ShaderManager,
    ) -> Self {
        // Create staging belt and a local pool
        let staging_belt = wgpu::util::StagingBelt::new(1024);
        let local_pool = futures::executor::LocalPool::new();

        let shaded_square = shaded_square::Pipeline::new(device, shaders);
        let shaded_round = shaded_round::Pipeline::new(device, shaders);
        let flat_round = flat_round::Pipeline::new(device, shaders);
        let custom = custom.build(&device, TEX_FORMAT, super::DEPTH_FORMAT);

        DrawPipe {
            local_pool,
            staging_belt,
            shaded_square,
            shaded_round,
            flat_round,
            custom,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, size: Size) -> DrawWindow<C::Window> {
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

        let rect = Rect::new(Coord::ZERO, size);

        let shaded_square = self.shaded_square.new_window(device, size, norm);
        let shaded_round = self.shaded_round.new_window(device, size, norm);
        let flat_round = self.flat_round.new_window(device, size);
        let custom = self.custom.new_window(device, size);

        // TODO: use extra caching so we don't load font for each window
        let font_data = kas::text::fonts::fonts().font_data();
        let mut fonts = Vec::with_capacity(font_data.len());
        for i in 0..font_data.len() {
            let (data, index) = font_data.get_data(i);
            fonts.push(FontRef::try_from_slice_and_index(data, index).unwrap());
        }
        let glyph_brush = GlyphBrushBuilder::using_fonts(fonts)
            .depth_stencil_state(super::GLPYH_DEPTH_DESC)
            .build(device, TEX_FORMAT);

        DrawWindow {
            depth: make_depth_texture(device, size),
            clip_regions: vec![rect],
            shaded_square,
            shaded_round,
            flat_round,
            custom,
            glyph_brush,
            dur_text: Default::default(),
        }
    }

    /// Process window resize
    pub fn resize(
        &self,
        window: &mut DrawWindow<C::Window>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: Size,
    ) {
        window.depth = make_depth_texture(device, size);
        window.clip_regions[0].size = size;
        window.shaded_square.resize(queue, size);
        window.shaded_round.resize(queue, size);
        self.custom.resize(&mut window.custom, device, queue, size);
        window.flat_round.resize(queue, size);
        queue.submit(std::iter::empty());
    }

    /// Render batched draw instructions via `rpass`
    pub fn render(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        device: &mut wgpu::Device,
        queue: &mut wgpu::Queue,
        frame_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
    ) {
        self.custom.update(&mut window.custom, device, queue);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render"),
        });

        let mut color_attachments = [wgpu::RenderPassColorAttachmentDescriptor {
            attachment: frame_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: true,
            },
        }];
        let mut depth_stencil_attachment = wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment: window.depth.as_ref().unwrap(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(kas_theme::START_PASS.depth()),
                store: true,
            }),
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0),
                store: true,
            }),
        };

        // We use a separate render pass for each clipped region.
        for (pass, rect) in window.clip_regions.iter().enumerate() {
            let ss = self
                .shaded_square
                .render_buf(&mut window.shaded_square, device, pass);
            let sr = self
                .shaded_round
                .render_buf(&mut window.shaded_round, device, pass);
            let fr = self
                .flat_round
                .render_buf(&mut window.flat_round, device, pass);

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("kas-wgpu render pass"),
                    color_attachments: &color_attachments,
                    depth_stencil_attachment: Some(depth_stencil_attachment.clone()),
                });
                rpass.set_scissor_rect(
                    rect.pos.0.cast(),
                    rect.pos.1.cast(),
                    rect.size.0.cast(),
                    rect.size.1.cast(),
                );

                ss.as_ref().map(|buf| buf.render(&mut rpass));
                sr.as_ref().map(|buf| buf.render(&mut rpass));
                fr.as_ref().map(|buf| buf.render(&mut rpass));
                self.custom
                    .render_pass(&mut window.custom, device, pass, &mut rpass);
            }

            color_attachments[0].ops.load = wgpu::LoadOp::Load;
            depth_stencil_attachment.depth_ops = Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            });
            depth_stencil_attachment.stencil_ops = Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true,
            });
        }

        // Fonts and custom pipes use their own render pass(es).
        let size = window.clip_regions[0].size;

        self.custom.render_final(
            &mut window.custom,
            device,
            &mut encoder,
            frame_view,
            depth_stencil_attachment.clone(),
            size,
        );

        window
            .glyph_brush
            .draw_queued(
                device,
                &mut self.staging_belt,
                &mut encoder,
                frame_view,
                depth_stencil_attachment,
                size.0.cast(),
                size.1.cast(),
            )
            .expect("glyph_brush.draw_queued");

        // Keep only first clip region (which is the entire window)
        window.clip_regions.truncate(1);

        self.staging_belt.finish();
        queue.submit(std::iter::once(encoder.finish()));

        // TODO: does this have to be after queue.submit?
        use futures::task::SpawnExt;
        self.local_pool
            .spawner()
            .spawn(self.staging_belt.recall())
            .expect("Recall staging belt");
        self.local_pool.run_until_stalled();
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
        let window_rect = self.clip_regions[0];
        let rect = rect.intersection(&window_rect).unwrap_or_else(|| {
            log::warn!("add_clip_region: intersection of rect and window rect is empty");
            Rect::new(Coord::ZERO, Size::ZERO)
        });
        let pass = self.clip_regions.len().cast();
        self.clip_regions.push(rect);
        Pass::new_pass_with_depth(pass, depth)
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
