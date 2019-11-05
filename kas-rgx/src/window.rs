// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use std::time::{Duration, Instant};

use rgx::core::*;
use wgpu_glyph::GlyphBrushBuilder;

use kas::event::Callback;
use kas::geom::Size;
use kas::{event, TkAction, WidgetId};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::render::Widgets;

/// Per-window data
pub struct Window {
    win: Box<dyn kas::Window>,
    /// The winit window
    pub(crate) ww: winit::window::Window,
    /// The renderer attached to this window
    rend: Renderer,
    swap_chain: SwapChain,
    pipeline: rgx::kit::shape2d::Pipeline,
    size: Size,
    timeouts: Vec<(usize, Instant, Option<Duration>)>,
    wrend: Widgets,
}

// Public functions, for use by the toolkit
impl Window {
    /// Construct a window
    pub fn new<T: 'static>(
        event_loop: &EventLoopWindowTarget<T>,
        mut win: Box<dyn kas::Window>,
    ) -> Result<Window, OsError> {
        win.enumerate(WidgetId::FIRST);

        let ww = winit::window::Window::new(event_loop)?;
        let dpi_factor = ww.hidpi_factor();
        let size: Size = ww.inner_size().to_physical(dpi_factor).into();

        let mut rend = Renderer::new(ww.raw_window_handle());
        let pipeline = rend.pipeline(Blending::default());
        let swap_chain = rend.swap_chain(size.0, size.1, PresentMode::default());

        let glyph_brush = GlyphBrushBuilder::using_font(crate::font::get_font())
            .build(rend.device.device_mut(), swap_chain.format());
        let mut wrend = Widgets::new(dpi_factor, glyph_brush);

        win.resize(&mut wrend, size);

        let w = Window {
            win,
            ww,
            rend,
            swap_chain,
            pipeline,
            size,
            timeouts: vec![],
            wrend,
        };

        Ok(w)
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    pub fn init(&mut self) -> Option<Instant> {
        self.ww.request_redraw();

        for (i, condition) in self.win.callbacks() {
            match condition {
                Callback::Start => {
                    self.win.trigger_callback(i, &mut self.wrend);
                }
                Callback::Repeat(dur) => {
                    self.win.trigger_callback(i, &mut self.wrend);
                    self.timeouts.push((i, Instant::now() + dur, Some(dur)));
                }
            }
        }

        self.next_resume()
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure(&mut self) {
        self.win.resize(&mut self.wrend, self.size);
        self.ww.request_redraw();
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
                self.do_resize(self.ww.inner_size());
            }
            event @ _ => event::Manager::handle_winit(&mut *self.win, &mut self.wrend, event),
        }
        self.wrend.pop_action()
    }

    pub(crate) fn timer_resume(&mut self, instant: Instant) -> (TkAction, Option<Instant>) {
        // Iterate over loop, mutating some elements, removing others.
        let mut i = 0;
        while i < self.timeouts.len() {
            for timeout in &mut self.timeouts[i..] {
                if timeout.1 == instant {
                    self.win.trigger_callback(timeout.0, &mut self.wrend);
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
        let size = size.to_physical(self.ww.hidpi_factor()).into();
        if size == self.size {
            return;
        }
        self.size = size;
        self.win.resize(&mut self.wrend, size);

        // Note: pipeline.resize relies on calling self.rend.update_pipeline
        // to avoid scaling issues; alternative is to create a new pipeline
        self.swap_chain = self.rend.swap_chain(size.0, size.1, PresentMode::default());
    }

    fn do_draw(&mut self) {
        let size = (self.swap_chain.width, self.swap_chain.height);
        let buffer = self.wrend.draw(&self.rend, size, &*self.win);

        let mut frame = self.rend.frame();
        self.rend
            .update_pipeline(&self.pipeline, rgx::kit::ortho(size.0, size.1), &mut frame);
        let texture = self.swap_chain.next();

        {
            let c = 0.2;
            let pass = &mut frame.pass(PassOp::Clear(Rgba::new(c, c, c, 1.0)), &texture);

            pass.set_pipeline(&self.pipeline);
            pass.draw_buffer(&buffer);
        }

        self.wrend
            .glyph_brush
            .draw_queued(
                self.rend.device.device_mut(),
                frame.encoder_mut(),
                texture.texture_view(),
                self.size.0,
                self.size.1,
            )
            .expect("glyph_brush.draw_queued");

        self.rend.present(frame);
    }
}
