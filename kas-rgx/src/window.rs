// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use rgx::core::*;
use rgx::math::Matrix4;
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder};

use kas::event::{Event, EventCoord, Response};
use kas::TkWidget;
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
    glyph_brush: GlyphBrush<'static, ()>,
    nums: (u32, u32), // TODO: is this useful?
    size: (u32, u32),
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
        let ww = winit::window::Window::new(event_loop)?;
        let mut rend = Renderer::new(ww.raw_window_handle());

        let size: (u32, u32) = ww.inner_size().to_physical(ww.hidpi_factor()).into();
        let pipeline = rend.pipeline(size.0, size.1, Blending::default());
        let swap_chain = rend.swap_chain(size.0, size.1, PresentMode::default());

        // FIXME: font source!
        let font: &[u8] = include_bytes!("/usr/share/fonts/dejavu/DejaVuSerif.ttf");
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(font)
            .build(rend.device.device_mut(), swap_chain.format());

        let num1 = win.enumerate(num0);

        let mut wrend = Widgets::new();

        win.resize(&mut wrend, size.into());

        let w = Window {
            win,
            ww,
            rend,
            swap_chain,
            pipeline,
            glyph_brush,
            nums: (num0, num1),
            size,
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
        self.win.on_start(&mut self.wrend);
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
                let coord: (i32, i32) = position.to_physical(self.ww.hidpi_factor()).into();
                let ev = EventCoord::CursorMoved {
                    device_id,
                    modifiers,
                };
                self.win
                    .handle(&mut self.wrend, Event::ToCoord(coord.into(), ev))
            }
            CursorLeft { .. } => {
                self.wrend.set_hover(None);
                return false;
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
}

// Internal functions
impl Window {
    fn do_resize(&mut self, size: LogicalSize) {
        let size: (u32, u32) = size.to_physical(self.ww.hidpi_factor()).into();
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
        let buffer = self
            .wrend
            .draw(&self.rend, &mut self.glyph_brush, size, &*self.win);

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

        self.glyph_brush
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
