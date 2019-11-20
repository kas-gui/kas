// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use std::time::{Duration, Instant};

use wgpu_glyph::GlyphBrushBuilder;

use kas::event::Callback;
use kas::geom::Size;
use kas::{event, TkAction};
use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::render::Widgets;

/// Per-window data
pub struct Window {
    widget: Box<dyn kas::Window>,
    /// The winit window
    pub(crate) window: winit::window::Window,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: Size,
    timeouts: Vec<(usize, Instant, Option<Duration>)>,
    wrend: Widgets,
}

// Public functions, for use by the toolkit
impl Window {
    /// Construct a window
    pub fn new<T: 'static>(
        event_loop: &EventLoopWindowTarget<T>,
        mut widget: Box<dyn kas::Window>,
    ) -> Result<Window, OsError> {
        let window = winit::window::Window::new(event_loop)?;
        let dpi_factor = window.hidpi_factor();
        let size: Size = window.inner_size().to_physical(dpi_factor).into();

        // TODO: move adapter, device and queue to Toolkit?
        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            backends: wgpu::BackendBit::PRIMARY,
        })
        .unwrap();

        let (mut device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let surface = wgpu::Surface::create(&window);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let glyph_brush = GlyphBrushBuilder::using_font(crate::font::get_font())
            .build(&mut device, sc_desc.format);

        let mut wrend = Widgets::new(dpi_factor, glyph_brush);
        wrend.ev_mgr.configure(widget.as_widget_mut());

        widget.resize(&mut wrend, size);

        let w = Window {
            widget,
            window,
            device,
            queue,
            surface,
            sc_desc,
            swap_chain,
            size, // TODO: remove?
            timeouts: vec![],
            wrend,
        };

        Ok(w)
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    pub fn init(&mut self) -> Option<Instant> {
        self.window.request_redraw();

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => {
                    self.widget.trigger_callback(i, &mut self.wrend);
                }
                Callback::Repeat(dur) => {
                    self.widget.trigger_callback(i, &mut self.wrend);
                    self.timeouts.push((i, Instant::now() + dur, Some(dur)));
                }
            }
        }

        self.next_resume()
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure(&mut self) {
        self.widget.resize(&mut self.wrend, self.size);
        self.window.request_redraw();
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event(&mut self, event: WindowEvent) -> TkAction {
        // Note: resize must be handled here to update self.swap_chain.
        match event {
            WindowEvent::Resized(size) => self.do_resize(size),
            WindowEvent::RedrawRequested => self.do_draw(),
            WindowEvent::HiDpiFactorChanged(factor) => {
                self.wrend.set_dpi_factor(factor);
                self.do_resize(self.window.inner_size());
            }
            event @ _ => event::Manager::handle_winit(&mut *self.widget, &mut self.wrend, event),
        }
        self.wrend.pop_action()
    }

    pub(crate) fn timer_resume(&mut self, instant: Instant) -> (TkAction, Option<Instant>) {
        // Iterate over loop, mutating some elements, removing others.
        let mut i = 0;
        while i < self.timeouts.len() {
            for timeout in &mut self.timeouts[i..] {
                if timeout.1 == instant {
                    self.widget.trigger_callback(timeout.0, &mut self.wrend);
                    if let Some(dur) = timeout.2 {
                        while timeout.1 <= Instant::now() {
                            timeout.1 += dur;
                        }
                    } else {
                        break; // remove
                    }
                }
                i += 1;
            }
            if i < self.timeouts.len() {
                self.timeouts.remove(i);
            }
        }

        (self.wrend.pop_action(), self.next_resume())
    }

    fn next_resume(&self) -> Option<Instant> {
        let mut next = None;
        for timeout in &self.timeouts {
            next = match next {
                None => Some(timeout.1),
                Some(t) => Some(t.min(timeout.1)),
            }
        }
        next
    }
}

// Internal functions
impl Window {
    fn do_resize(&mut self, size: LogicalSize) {
        let size = size.to_physical(self.window.hidpi_factor()).into();
        if size == self.size {
            return;
        }
        self.size = size;
        self.widget.resize(&mut self.wrend, size);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn do_draw(&mut self) {
        let frame = self.swap_chain.get_next_texture();

        let c = 0.2;
        let rpass_color_attachment = wgpu::RenderPassColorAttachmentDescriptor {
            attachment: &frame.view,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color {
                r: c,
                g: c,
                b: c,
                a: 1.0,
            },
        };

        let desc = wgpu::CommandEncoderDescriptor { todo: 0 };
        let mut encoder = self.device.create_command_encoder(&desc);

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[rpass_color_attachment],
            depth_stencil_attachment: None,
        });

        self.wrend.draw(&mut rpass, self.size, &*self.widget);

        drop(rpass);

        self.wrend
            .glyph_brush
            .draw_queued(
                &mut self.device,
                &mut encoder,
                &frame.view,
                self.size.0,
                self.size.1,
            )
            .expect("glyph_brush.draw_queued");

        self.queue.submit(&[encoder.finish()]);
    }
}
