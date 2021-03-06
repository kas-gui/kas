// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`

use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;

use super::*;
use kas::cast::Cast;
use kas::draw::color::Rgba;
use kas::draw::*;
use kas::geom::{Coord, Quad, Rect, Size, Vec2};
use kas::text::{Effect, TextDisplay};

impl<C: CustomPipe> DrawPipe<C> {
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        mut custom: CB,
        (device, queue): (wgpu::Device, wgpu::Queue),
        raster_config: &kas_theme::RasterConfig,
    ) -> Self {
        let shaders = ShaderManager::new(&device);

        // Create staging belt and a local pool
        let staging_belt = wgpu::util::StagingBelt::new(1024);
        let local_pool = futures::executor::LocalPool::new();

        let bgl_common = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("common bind group layout"),
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

        let images = images::Images::new(&device, &shaders, &bgl_common);
        let shaded_square = shaded_square::Pipeline::new(&device, &shaders, &bgl_common);
        let shaded_round = shaded_round::Pipeline::new(&device, &shaders, &bgl_common);
        let flat_round = flat_round::Pipeline::new(&device, &shaders, &bgl_common);
        let custom = custom.build(&device, &bgl_common, RENDER_TEX_FORMAT);
        let text = text_pipe::Pipeline::new(&device, &shaders, &bgl_common, raster_config);

        DrawPipe {
            device,
            queue,
            local_pool,
            staging_belt,
            bgl_common,
            images,
            shaded_square,
            shaded_round,
            flat_round,
            custom,
            text,
        }
    }

    /// Construct per-window state
    pub fn new_window(&self, size: Size) -> DrawWindow<C::Window> {
        type Scale = [f32; 2];
        let scale_factor: Scale = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        let scale_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        let light_norm_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SR light_norm_buf"),
                contents: bytemuck::cast_slice(&light_norm),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });

        let bg_common = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("common bind group"),
            layout: &self.bgl_common,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &scale_buf,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_norm_buf,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let rect = Rect::new(Coord::ZERO, size);

        let custom = self.custom.new_window(&self.device, size);

        DrawWindow {
            scale_buf,
            clip_regions: vec![rect],
            bg_common,
            images: Default::default(),
            shaded_square: Default::default(),
            shaded_round: Default::default(),
            flat_round: Default::default(),
            custom,
            text: Default::default(),
        }
    }

    /// Wraps [`wgpu::Device::create_swap_chain`]
    pub fn create_swap_chain(
        &self,
        surface: &wgpu::Surface,
        desc: &wgpu::SwapChainDescriptor,
    ) -> wgpu::SwapChain {
        self.device.create_swap_chain(surface, desc)
    }

    /// Process window resize
    pub fn resize(&self, window: &mut DrawWindow<C::Window>, size: Size) {
        window.clip_regions[0].size = size;

        let scale_factor = [2.0 / size.0 as f32, -2.0 / size.1 as f32];
        self.queue
            .write_buffer(&window.scale_buf, 0, bytemuck::cast_slice(&scale_factor));

        self.custom
            .resize(&mut window.custom, &self.device, &self.queue, size);

        self.queue.submit(std::iter::empty());
    }

    /// Render batched draw instructions via `rpass`
    pub fn render(
        &mut self,
        window: &mut DrawWindow<C::Window>,
        frame_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render"),
            });

        self.images.prepare(
            &mut window.images,
            &self.device,
            &mut self.staging_belt,
            &mut encoder,
        );
        window
            .shaded_square
            .write_buffers(&self.device, &mut self.staging_belt, &mut encoder);
        window
            .shaded_round
            .write_buffers(&self.device, &mut self.staging_belt, &mut encoder);
        window
            .flat_round
            .write_buffers(&self.device, &mut self.staging_belt, &mut encoder);
        self.custom.prepare(
            &mut window.custom,
            &self.device,
            &mut self.staging_belt,
            &mut encoder,
        );
        self.text.prepare(&self.device, &self.queue);
        window
            .text
            .write_buffers(&self.device, &mut self.staging_belt, &mut encoder);

        let mut color_attachments = [wgpu::RenderPassColorAttachment {
            view: frame_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: true,
            },
        }];

        // We use a separate render pass for each clipped region.
        for (pass, rect) in window.clip_regions.iter().enumerate() {
            if rect.size.0 == 0 || rect.size.1 == 0 {
                continue;
            }
            let bg_common = &window.bg_common;

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

                self.images
                    .render(&window.images, pass, &mut rpass, bg_common);
                self.shaded_square
                    .render(&window.shaded_square, pass, &mut rpass, bg_common);
                self.shaded_round
                    .render(&window.shaded_round, pass, &mut rpass, bg_common);
                self.flat_round
                    .render(&window.flat_round, pass, &mut rpass, bg_common);
                self.custom.render_pass(
                    &mut window.custom,
                    &self.device,
                    pass,
                    &mut rpass,
                    bg_common,
                );
                self.text.render(&window.text, pass, &mut rpass, bg_common);
            }

            color_attachments[0].ops.load = wgpu::LoadOp::Load;
        }

        // Fonts and custom pipes use their own render pass(es).
        let size = window.clip_regions[0].size;

        self.custom.render_final(
            &mut window.custom,
            &self.device,
            &mut encoder,
            frame_view,
            size,
        );

        // Keep only first clip region (which is the entire window)
        window.clip_regions.truncate(1);

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));

        use futures::task::SpawnExt;
        self.local_pool
            .spawner()
            .spawn(self.staging_belt.recall())
            .expect("Recall staging belt");
        self.local_pool.run_until_stalled();
    }
}

impl<C: CustomPipe> DrawableShared for DrawPipe<C> {
    type Draw = DrawWindow<C::Window>;

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[inline]
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, ImageError> {
        self.images.alloc(size)
    }

    #[inline]
    fn image_upload(&mut self, id: ImageId, data: &[u8], format: ImageFormat) {
        self.images
            .upload(&self.device, &self.queue, id, data, format);
    }

    #[inline]
    fn image_free(&mut self, id: ImageId) {
        self.images.free(id);
    }

    #[inline]
    fn image_size(&self, id: ImageId) -> Option<(u32, u32)> {
        self.images.image_size(id)
    }

    #[inline]
    fn draw_image(&self, target: Draw<Self::Draw>, id: ImageId, rect: Quad) {
        if let Some((atlas, tex)) = self.images.get_im_atlas_coords(id) {
            target.draw.images.rect(target.pass(), atlas, tex, rect);
        };
    }

    #[inline]
    fn draw_text(&mut self, target: Draw<Self::Draw>, pos: Vec2, text: &TextDisplay, col: Rgba) {
        target
            .draw
            .text
            .text(&mut self.text, target.pass(), pos, text, col);
    }

    fn draw_text_col_effects(
        &mut self,
        target: Draw<Self::Draw>,
        pos: Vec2,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) {
        let pass = target.pass();
        let rects =
            target
                .draw
                .text
                .text_col_effects(&mut self.text, pass, pos, text, col, effects);
        for rect in rects {
            target.draw.shaded_square.rect(pass, rect, col);
        }
    }

    fn draw_text_effects(
        &mut self,
        target: Draw<Self::Draw>,
        pos: Vec2,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    ) {
        let pass = target.pass();
        let rects = target
            .draw
            .text
            .text_effects(&mut self.text, pass, pos, text, effects);
        for (rect, col) in rects {
            target.draw.shaded_square.rect(pass, rect, col);
        }
    }
}

impl<CW: CustomWindow> Drawable for DrawWindow<CW> {
    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[inline]
    fn as_drawable_mut(&mut self) -> &mut dyn Drawable {
        self
    }

    fn add_clip_region(&mut self, pass: Pass, rect: Rect, class: RegionClass) -> Pass {
        let parent = match class {
            RegionClass::ScrollRegion => pass.pass(),
            RegionClass::Overlay => 0,
        };
        let parent_rect = self.clip_regions[parent];
        let rect = rect.intersection(&parent_rect).unwrap_or(Rect::ZERO);
        let pass = self.clip_regions.len().cast();
        self.clip_regions.push(rect);
        Pass::new(pass)
    }

    #[inline]
    fn get_clip_rect(&self, pass: Pass) -> Rect {
        self.clip_regions[pass.pass()]
    }

    #[inline]
    fn rect(&mut self, pass: Pass, rect: Quad, col: Rgba) {
        self.shaded_square.rect(pass, rect, col);
    }

    #[inline]
    fn frame(&mut self, pass: Pass, outer: Quad, inner: Quad, col: Rgba) {
        self.shaded_square.frame(pass, outer, inner, col);
    }
}

impl<CW: CustomWindow> DrawableRounded for DrawWindow<CW> {
    #[inline]
    fn rounded_line(&mut self, pass: Pass, p1: Vec2, p2: Vec2, radius: f32, col: Rgba) {
        self.flat_round.line(pass, p1, p2, radius, col);
    }

    #[inline]
    fn circle(&mut self, pass: Pass, rect: Quad, inner_radius: f32, col: Rgba) {
        self.flat_round.circle(pass, rect, inner_radius, col);
    }

    #[inline]
    fn rounded_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        inner_radius: f32,
        col: Rgba,
    ) {
        self.flat_round
            .rounded_frame(pass, outer, inner, inner_radius, col);
    }
}

impl<CW: CustomWindow> DrawableShaded for DrawWindow<CW> {
    #[inline]
    fn shaded_square(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.shaded_square
            .shaded_rect(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_circle(&mut self, pass: Pass, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.shaded_round.circle(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_square_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        outer_col: Rgba,
        inner_col: Rgba,
    ) {
        self.shaded_square
            .shaded_frame(pass, outer, inner, Vec2::from(norm), outer_col, inner_col);
    }

    #[inline]
    fn shaded_round_frame(
        &mut self,
        pass: Pass,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Rgba,
    ) {
        self.shaded_round
            .shaded_frame(pass, outer, inner, Vec2::from(norm), col);
    }
}
