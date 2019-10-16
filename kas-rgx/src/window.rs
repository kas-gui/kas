// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use rgx::core::*;
use rgx::math::Matrix4;

use kas::event::{Event, EventCoord, Response};
use kas::WidgetId;
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
    //     /// The GTK window
    //     pub gwin: gtk::Window,
    nums: (u32, u32), // TODO: is this useful?
    widgets: Widgets,
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
        let rend = Renderer::new(ww.raw_window_handle());

        let size: (u32, u32) = ww.inner_size().to_physical(ww.hidpi_factor()).into();
        let pipeline = rend.pipeline(size.0, size.1, Blending::default());
        let swap_chain = rend.swap_chain(size.0, size.1, PresentMode::default());

        let num1 = win.enumerate(num0);

        let mut widgets = Widgets::new();

        win.configure_widgets(&mut widgets);
        win.resize(&mut widgets, size.into());

        let w = Window {
            win,
            ww,
            rend,
            swap_chain,
            pipeline,
            nums: (num0, num1),
            widgets,
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
        self.win.on_start(&mut self.widgets);
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
                self.win
                    .handle(&mut self.widgets, Event::ToCoord(coord, ev))
            }
            CursorLeft { .. } => {
                self.set_hover(None);
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

        match response {
            Response::None | Response::Msg(()) => false,
            // TODO: handle Exit properly
            Response::Close | Response::Exit => true,
            Response::Hover(id) => {
                self.set_hover(Some(id));
                false
            }
        }
    }

    fn set_hover(&mut self, hover: Option<WidgetId>) {
        if self.widgets.hover != hover {
            println!("Hover widget: {:?}", hover);
            self.widgets.hover = hover;
            self.ww.request_redraw();
        }
    }
}

// Internal functions
impl Window {
    fn do_resize(&mut self, size: LogicalSize) {
        let size: (u32, u32) = size.to_physical(self.ww.hidpi_factor()).into();
        if size == (self.swap_chain.width, self.swap_chain.height) {
            return;
        }

        // Note: pipeline.resize relies on calling self.rend.update_pipeline
        // to avoid scaling issues; alternative is to create a new pipeline
        self.pipeline.resize(size.0, size.1);
        self.swap_chain = self.rend.swap_chain(size.0, size.1, PresentMode::default());

        // TODO: work with logical size to allow DPI scaling
        self.win.configure_widgets(&mut self.widgets);
        self.win.resize(&mut self.widgets, size.into());
    }

    fn do_draw(&mut self) {
        let size = (self.swap_chain.width, self.swap_chain.height);
        let buffer = self.widgets.draw(&self.rend, size, &*self.win);

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
        self.rend.submit(frame);
    }
}
