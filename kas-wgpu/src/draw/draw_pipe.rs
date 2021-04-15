// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`

use std::any::Any;
use std::f32::consts::FRAC_PI_2;
use std::path::Path;
use wgpu::util::DeviceExt;
use wgpu_glyph::{ab_glyph::FontRef, GlyphBrushBuilder};

use super::{
    flat_round, images, shaded_round, shaded_square, CustomPipe, CustomPipeBuilder, CustomWindow,
    DrawPipe, DrawWindow, ShaderManager, TEX_FORMAT,
};
use kas::cast::Cast;
use kas::draw::{Colour, Draw, DrawRounded, DrawShaded, DrawShared, Pass};
use kas::geom::{Coord, Quad, Rect, Size, Vec2};

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

        let images = images::Pipeline::new(device, shaders);
        let shaded_square = shaded_square::Pipeline::new(device, shaders);
        let shaded_round = shaded_round::Pipeline::new(device, shaders);
        let flat_round = flat_round::Pipeline::new(device, shaders);
        let custom = custom.build(&device, TEX_FORMAT);

        DrawPipe {
            local_pool,
            staging_belt,
            images,
            shaded_square,
            shaded_round,
            flat_round,
            custom,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, device: &wgpu::Device, size: Size) -> DrawWindow<C::Window> {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SR scale_buf"),
            contents: bytemuck::cast_slice(&scale_factor),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        // Light dir: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
        // normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
        // (from UP, clockwise), both in radians.
        let dir: (f32, f32) = (0.3, 0.4);
        assert!(dir.0 >= 0.0);
        assert!(dir.0 < FRAC_PI_2);
        let a = (dir.0.sin(), dir.0.cos());
        // We normalise intensity:
        let f = a.0 / a.1;
        let light_norm = [dir.1.sin() * f, -dir.1.cos() * f, 1.0];

        let light_norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SR light_norm_buf"),
            contents: bytemuck::cast_slice(&light_norm),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let rect = Rect::new(Coord::ZERO, size);

        let images = self.images.new_window(device, &scale_buf);
        let shaded_square = self
            .shaded_square
            .new_window(device, &scale_buf, &light_norm_buf);
        let shaded_round = self
            .shaded_round
            .new_window(device, &scale_buf, &light_norm_buf);
        let flat_round = self.flat_round.new_window(device, &scale_buf);
        let custom = self.custom.new_window(device, &scale_buf, size);

        // TODO: use extra caching so we don't load font for each window
        let font_data = kas::text::fonts::fonts().font_data();
        let mut fonts = Vec::with_capacity(font_data.len());
        for i in 0..font_data.len() {
            let (data, index) = font_data.get_data(i);
            fonts.push(FontRef::try_from_slice_and_index(data, index).unwrap());
        }
        let glyph_brush = GlyphBrushBuilder::using_fonts(fonts).build(device, TEX_FORMAT);

        DrawWindow {
            scale_buf,
            clip_regions: vec![rect],
            images,
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
        window.clip_regions[0].size = size;

        let scale_factor = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        queue.write_buffer(&window.scale_buf, 0, bytemuck::cast_slice(&scale_factor));

        self.custom.resize(&mut window.custom, device, queue, size);

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
        // TODO: could potentially start preparing images asynchronously after
        // configure, then join thread and do any final prep now.
        self.images.prepare(device, queue);

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

        // We use a separate render pass for each clipped region.
        for (pass, rect) in window.clip_regions.iter().enumerate() {
            let im = self.images.render_buf(&mut window.images, device, pass);
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
                    depth_stencil_attachment: None,
                });
                rpass.set_scissor_rect(
                    rect.pos.0.cast(),
                    rect.pos.1.cast(),
                    rect.size.0.cast(),
                    rect.size.1.cast(),
                );

                im.as_ref().map(|buf| buf.render(&mut rpass));
                ss.as_ref().map(|buf| buf.render(&mut rpass));
                sr.as_ref().map(|buf| buf.render(&mut rpass));
                fr.as_ref().map(|buf| buf.render(&mut rpass));
                self.custom
                    .render_pass(&mut window.custom, device, pass, &mut rpass);
            }

            color_attachments[0].ops.load = wgpu::LoadOp::Load;
        }

        // Fonts and custom pipes use their own render pass(es).
        let size = window.clip_regions[0].size;

        self.custom
            .render_final(&mut window.custom, device, &mut encoder, frame_view, size);

        window
            .glyph_brush
            .draw_queued(
                device,
                &mut self.staging_belt,
                &mut encoder,
                frame_view,
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

    #[inline]
    fn load_image(&mut self, path: &Path) {
        self.images.load(path);
    }

    #[inline]
    fn image_size(&self) -> Size {
        self.images.image_size().into()
    }

    #[inline]
    fn draw_image(&self, window: &mut Self::Draw, pass: Pass, rect: Quad) {
        window.images.rect(&self.images, pass, rect);
    }
}

impl<CW: CustomWindow> Draw for DrawWindow<CW> {
    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn add_clip_region(&mut self, rect: Rect) -> Pass {
        let window_rect = self.clip_regions[0];
        let rect = rect.intersection(&window_rect).unwrap_or_else(|| {
            log::warn!("add_clip_region: intersection of rect and window rect is empty");
            Rect::new(Coord::ZERO, Size::ZERO)
        });
        let pass = self.clip_regions.len().cast();
        self.clip_regions.push(rect);
        Pass::new(pass)
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

impl<CW: CustomWindow> DrawRounded for DrawWindow<CW> {
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

impl<CW: CustomWindow> DrawShaded for DrawWindow<CW> {
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
