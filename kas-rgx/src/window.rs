// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use std::time::{Duration, Instant};

use rgx::core::*;
use rgx::math::Matrix4;
use wgpu_glyph::GlyphBrushBuilder;

use kas::callback::Condition;
use kas::event::{Event, EventChild, EventCoord, Response};
use kas::{Size, TkWidget};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event::{ElementState, MouseButton};
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
    nums: (u32, u32), // TODO: is this useful?
    size: Size,
    timeouts: Vec<(usize, Instant, Option<Duration>)>,
    wrend: Widgets,
}

// Public functions, for use by the toolkit
impl Window {
    /// Construct a window
    ///
    /// Parameter `num0`: for the first window, use 0. For any other window,
    /// use the previous window's `nums().1` value.
    pub fn new<T: 'static>(
        event_loop: &EventLoopWindowTarget<T>,
        mut win: Box<dyn kas::Window>,
        num0: u32,
    ) -> Result<Window, OsError> {
        let num1 = win.enumerate(num0);

        let ww = winit::window::Window::new(event_loop)?;
        let dpi_factor = ww.hidpi_factor();
        let size: Size = ww.inner_size().to_physical(dpi_factor).into();

        let mut rend = Renderer::new(ww.raw_window_handle());
        let pipeline = rend.pipeline(size.0, size.1, Blending::default());
        let swap_chain = rend.swap_chain(size.0, size.1, PresentMode::default());

        let glyph_brush = GlyphBrushBuilder::using_font(crate::font::get_font())
            .build(rend.device.device_mut(), swap_chain.format());
        let mut wrend = Widgets::new(glyph_brush);

        win.resize(&mut wrend, size.into());

        let w = Window {
            win,
            ww,
            rend,
            swap_chain,
            pipeline,
            nums: (num0, num1),
            size,
            timeouts: vec![],
            wrend,
        };

        Ok(w)
    }

    /// Range of widget numbers used, from first to last+1.
    pub fn nums(&self) -> (u32, u32) {
        self.nums
    }

    /// Called by the `Toolkit` just before the event loop starts to initialise
    /// windows.
    pub fn prepare(&mut self) {
        self.ww.request_redraw();

        for (i, condition) in self.win.callbacks() {
            match condition {
                Condition::Start => {
                    self.win.trigger_callback(i, &mut self.wrend);
                }
                Condition::Repeat(dur) => {
                    self.win.trigger_callback(i, &mut self.wrend);
                    self.timeouts.push((i, Instant::now() + dur, Some(dur)));
                }
            }
        }
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event(&mut self, event: WindowEvent) -> bool {
        use WindowEvent::*;
        let response: Response<()> = match event {
            Resized(size) => {
                self.do_resize(size);
                return false;
            }
            CloseRequested => {
                return true;
            }
            CursorMoved {
                device_id,
                position,
                modifiers,
            } => {
                let coord = position.to_physical(self.ww.hidpi_factor()).into();
                let ev = EventCoord::CursorMoved {
                    device_id,
                    modifiers,
                };
                self.win.handle(&mut self.wrend, Event::ToCoord(coord, ev))
            }
            CursorLeft { .. } => {
                self.wrend.set_hover(None);
                return false;
            }
            MouseInput {
                device_id,
                state,
                button,
                modifiers,
            } => {
                let ev = EventChild::MouseInput {
                    device_id,
                    state,
                    button,
                    modifiers,
                };
                if let Some(id) = self.wrend.hover() {
                    self.win.handle(&mut self.wrend, Event::ToChild(id, ev))
                } else {
                    // This happens for example on click-release when the
                    // cursor is no longer over the window.
                    // TODO: move event handler
                    if button == MouseButton::Left && state == ElementState::Released {
                        self.wrend.set_click_start(None);
                    }
                    Response::None
                }
            }
            RedrawRequested => {
                self.do_draw();
                return false;
            }
            HiDpiFactorChanged(_) => {
                self.do_resize(self.ww.inner_size());
                return false;
            }
            _ => {
                //                 println!("Unhandled window event: {:?}", event);
                return false;
            }
        };

        // Event handling may trigger a redraw
        if self.wrend.need_redraw() {
            self.ww.request_redraw();
        }

        match response {
            Response::None | Response::Msg(()) => false,
            // TODO: handle Exit properly
            Response::Close | Response::Exit => true,
        }
    }

    pub(crate) fn timer_resume(&mut self, instant: Instant) {
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

        // Timer handling may trigger a redraw
        if self.wrend.need_redraw() {
            self.ww.request_redraw();
        }
    }

    pub(crate) fn next_resume(&self) -> Option<Instant> {
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

        // Note: pipeline.resize relies on calling self.rend.update_pipeline
        // to avoid scaling issues; alternative is to create a new pipeline
        self.pipeline.resize(size.0, size.1);
        self.swap_chain = self.rend.swap_chain(size.0, size.1, PresentMode::default());

        // TODO: work with logical size to allow DPI scaling
        self.win.resize(&mut self.wrend, size.into());
    }

    fn do_draw(&mut self) {
        let size = (self.swap_chain.width, self.swap_chain.height);
        let buffer = self.wrend.draw(&self.rend, size, &*self.win);

        let mut frame = self.rend.frame();
        self.rend
            .update_pipeline(&self.pipeline, Matrix4::identity(), &mut frame);
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

        self.rend.submit(frame);
    }
}
