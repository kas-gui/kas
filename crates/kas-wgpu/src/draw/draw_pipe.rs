// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Drawing API for `kas_wgpu`

use std::f32::consts::FRAC_PI_2;
use wgpu::util::DeviceExt;

use super::*;
use crate::DrawShadedImpl;
use crate::{Error, Options};
use kas::cast::traits::*;
use kas::draw::color::Rgba;
use kas::draw::*;
use kas::geom::{Quad, Rect, Size, Vec2};
use kas::text::{Effect, TextDisplay};

impl<C: CustomPipe> DrawPipe<C> {
    /// Construct
    pub fn new<CB: CustomPipeBuilder<Pipe = C>>(
        mut custom: CB,
        options: &Options,
        raster_config: &kas::theme::RasterConfig,
    ) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(options.backend());
        let adapter_options = options.adapter_options();
        let req = instance.request_adapter(&adapter_options);
        let adapter = match futures::executor::block_on(req) {
            Some(a) => a,
            None => return Err(Error::NoAdapter),
        };
        log::info!("Using graphics adapter: {}", adapter.get_info().name);

        let desc = CB::device_descriptor();
        let trace_path = options.wgpu_trace_path.as_deref();
        let req = adapter.request_device(&desc, trace_path);
        let (device, queue) = futures::executor::block_on(req)?;

        let shaders = ShaderManager::new(&device);

        // Create staging belt and a local pool
        let staging_belt = wgpu::util::StagingBelt::new(1024);

        let bgl_common = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("common bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });

        // Light dir: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
        // normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
        // (from UP, clockwise), both in radians.
        let dir: (f32, f32) = (0.3, 0.4);
        assert!(0.0 <= dir.0 && dir.0 < FRAC_PI_2);
        let a = (dir.0.sin(), dir.0.cos());
        // We normalise intensity:
        let f = a.0 / a.1;
        let light_norm = [dir.1.sin() * f, -dir.1.cos() * f, 1.0, 0.0];

        let light_norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light_norm_buf"),
            contents: bytemuck::cast_slice(&light_norm),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let images = images::Images::new(&device, &shaders, &bgl_common);
        let shaded_square = shaded_square::Pipeline::new(&device, &shaders, &bgl_common);
        let shaded_round = shaded_round::Pipeline::new(&device, &shaders, &bgl_common);
        let flat_round = flat_round::Pipeline::new(&device, &shaders, &bgl_common);
        let round_2col = round_2col::Pipeline::new(&device, &shaders, &bgl_common);
        let custom = custom.build(&device, &bgl_common, RENDER_TEX_FORMAT);
        let text = text_pipe::Pipeline::new(&device, &shaders, &bgl_common, raster_config);

        Ok(DrawPipe {
            instance,
            device,
            queue,
            staging_belt,
            bgl_common,
            light_norm_buf,
            bg_common: vec![],
            images,
            shaded_square,
            shaded_round,
            flat_round,
            round_2col,
            custom,
            text,
        })
    }

    /// Construct per-window state
    pub fn new_window(&self) -> DrawWindow<C::Window> {
        let custom = self.custom.new_window(&self.device);

        DrawWindow {
            animation: AnimationState::None,
            scale: Default::default(),
            clip_regions: vec![Default::default()],
            images: Default::default(),
            shaded_square: Default::default(),
            shaded_round: Default::default(),
            flat_round: Default::default(),
            round_2col: Default::default(),
            custom,
            text: Default::default(),
        }
    }

    /// Process window resize
    pub fn resize(&self, window: &mut DrawWindow<C::Window>, size: Size) {
        window.clip_regions[0].0.size = size;

        let vsize = Vec2::conv(size);
        let off = vsize * -0.5;
        let scale = 2.0 / vsize;
        window.scale = [off.0, off.1, scale.0, -scale.1];

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
        // Update all bind groups. We use a separate bind group for each clip
        // region and update on each render, although they don't always change.
        // NOTE: we could use push constants instead.
        let mut scale = window.scale;
        let base_offset = (scale[0], scale[1]);
        for (region, bg) in window.clip_regions.iter().zip(self.bg_common.iter()) {
            let offset = Vec2::conv(region.1);
            scale[0] = base_offset.0 - offset.0;
            scale[1] = base_offset.1 - offset.1;
            self.queue
                .write_buffer(&bg.0, 0, bytemuck::cast_slice(&scale));
        }
        let device = &self.device;
        let bg_len = self.bg_common.len();
        if window.clip_regions.len() > bg_len {
            let (bgl_common, light_norm_buf) = (&self.bgl_common, &self.light_norm_buf);
            self.bg_common
                .extend(window.clip_regions[bg_len..].iter().map(|region| {
                    let offset = Vec2::conv(region.1);
                    scale[0] = base_offset.0 - offset.0;
                    scale[1] = base_offset.1 - offset.1;
                    let scale_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("scale_buf"),
                        contents: bytemuck::cast_slice(&scale),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });
                    let bg_common = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("common bind group"),
                        layout: bgl_common,
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
                                    buffer: light_norm_buf,
                                    offset: 0,
                                    size: None,
                                }),
                            },
                        ],
                    });
                    (scale_buf, bg_common)
                }));
        }
        self.queue.submit(std::iter::empty());

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
        window
            .round_2col
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

        let mut color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: frame_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: true,
            },
        })];

        // We use a separate render pass for each clipped region.
        for (pass, (rect, _)) in window.clip_regions.iter().enumerate() {
            if rect.size.0 == 0 || rect.size.1 == 0 {
                continue;
            }
            let bg_common = &self.bg_common[pass].1;

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

                self.round_2col
                    .render(&window.round_2col, pass, &mut rpass, bg_common);
                self.shaded_square
                    .render(&window.shaded_square, pass, &mut rpass, bg_common);
                self.images
                    .render(&window.images, pass, &mut rpass, bg_common);
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

            color_attachments[0].as_mut().unwrap().ops.load = wgpu::LoadOp::Load;
        }

        let size = window.clip_regions[0].0.size;

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

        self.staging_belt.recall();
    }
}

impl<C: CustomPipe> DrawSharedImpl for DrawPipe<C> {
    type Draw = DrawWindow<C::Window>;

    #[inline]
    fn image_alloc(&mut self, size: (u32, u32)) -> Result<ImageId, AllocError> {
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
    fn draw_image(&self, draw: &mut Self::Draw, pass: PassId, id: ImageId, rect: Quad) {
        if let Some((atlas, tex)) = self.images.get_im_atlas_coords(id) {
            draw.images.rect(pass, atlas, tex, rect);
        };
    }

    #[inline]
    fn draw_text(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
    ) {
        draw.text.text(&mut self.text, pass, rect, text, col);
    }

    fn draw_text_effects(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        col: Rgba,
        effects: &[Effect<()>],
    ) {
        let rects = draw
            .text
            .text_effects(&mut self.text, pass, rect, text, col, effects);
        for rect in rects {
            draw.shaded_square.rect(pass, rect, col);
        }
    }

    fn draw_text_effects_rgba(
        &mut self,
        draw: &mut Self::Draw,
        pass: PassId,
        rect: Rect,
        text: &TextDisplay,
        effects: &[Effect<Rgba>],
    ) {
        let rects = draw
            .text
            .text_effects_rgba(&mut self.text, pass, rect, text, effects);
        for (rect, col) in rects {
            draw.shaded_square.rect(pass, rect, col);
        }
    }
}

impl<CW: CustomWindow> DrawImpl for DrawWindow<CW> {
    fn animation_mut(&mut self) -> &mut AnimationState {
        &mut self.animation
    }

    fn new_pass(
        &mut self,
        parent_pass: PassId,
        rect: Rect,
        offset: Offset,
        class: PassType,
    ) -> PassId {
        let parent = match class {
            PassType::Clip => &self.clip_regions[parent_pass.pass()],
            PassType::Overlay => &self.clip_regions[0],
        };
        let rect = rect - parent.1;
        let offset = offset + parent.1;
        let rect = rect.intersection(&parent.0).unwrap_or(Rect::ZERO);
        let pass = self.clip_regions.len().cast();
        self.clip_regions.push((rect, offset));
        PassId::new(pass)
    }

    #[inline]
    fn get_clip_rect(&self, pass: PassId) -> Rect {
        let region = &self.clip_regions[pass.pass()];
        region.0 + region.1
    }

    #[inline]
    fn rect(&mut self, pass: PassId, rect: Quad, col: Rgba) {
        self.shaded_square.rect(pass, rect, col);
    }

    #[inline]
    fn frame(&mut self, pass: PassId, outer: Quad, inner: Quad, col: Rgba) {
        self.shaded_square.frame(pass, outer, inner, col);
    }
}

impl<CW: CustomWindow> DrawRoundedImpl for DrawWindow<CW> {
    #[inline]
    fn rounded_line(&mut self, pass: PassId, p1: Vec2, p2: Vec2, radius: f32, col: Rgba) {
        self.flat_round.line(pass, p1, p2, radius, col);
    }

    #[inline]
    fn circle(&mut self, pass: PassId, rect: Quad, inner_radius: f32, col: Rgba) {
        self.flat_round.circle(pass, rect, inner_radius, col);
    }

    #[inline]
    fn circle_2col(&mut self, pass: PassId, rect: Quad, col1: Rgba, col2: Rgba) {
        self.round_2col.circle(pass, rect, col1, col2);
    }

    #[inline]
    fn rounded_frame(&mut self, pass: PassId, outer: Quad, inner: Quad, r1: f32, col: Rgba) {
        self.flat_round.rounded_frame(pass, outer, inner, r1, col);
    }

    #[inline]
    fn rounded_frame_2col(&mut self, pass: PassId, outer: Quad, inner: Quad, c1: Rgba, c2: Rgba) {
        self.round_2col.frame(pass, outer, inner, c1, c2);
    }
}

impl<CW: CustomWindow> DrawShadedImpl for DrawWindow<CW> {
    #[inline]
    fn shaded_square(&mut self, pass: PassId, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.shaded_square
            .shaded_rect(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_circle(&mut self, pass: PassId, rect: Quad, norm: (f32, f32), col: Rgba) {
        self.shaded_round.circle(pass, rect, Vec2::from(norm), col);
    }

    #[inline]
    fn shaded_square_frame(
        &mut self,
        pass: PassId,
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
        pass: PassId,
        outer: Quad,
        inner: Quad,
        norm: (f32, f32),
        col: Rgba,
    ) {
        self.shaded_round
            .shaded_frame(pass, outer, inner, Vec2::from(norm), col);
    }
}
