// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use log::{debug, info, trace};
use std::time::Instant;

use kas::event::{Callback, ManagerState, UpdateHandle};
use kas::geom::{Coord, Rect, Size};
use kas::{theme, TkAction};
use winit::dpi::PhysicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::draw::DrawPipe;
use crate::shared::SharedState;
use crate::ProxyAction;

/// Per-window data
pub(crate) struct Window<TW> {
    widget: Box<dyn kas::Window>,
    mgr: ManagerState,
    /// The winit window
    pub(crate) window: winit::window::Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    draw_pipe: DrawPipe,
    theme_window: TW,
}

// Public functions, for use by the toolkit
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    /// Construct a window
    pub fn new<T: theme::Theme<DrawPipe, Window = TW>>(
        shared: &mut SharedState<T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        widget: Box<dyn kas::Window>,
    ) -> Result<Self, OsError> {
        let window = winit::window::Window::new(elwt)?;
        window.set_title(widget.title());

        let dpi_factor = window.scale_factor();
        let size: Size = window.inner_size().into();
        info!("Constucted new window with size {:?}", size);

        let surface = wgpu::Surface::create(&window);

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mut draw_pipe = DrawPipe::new(shared, sc_desc.format, size);
        let theme_window = shared.theme.new_window(&mut draw_pipe, dpi_factor as f32);

        let mgr = ManagerState::new(dpi_factor);

        Ok(Window {
            widget,
            mgr,
            window,
            surface,
            sc_desc,
            swap_chain,
            draw_pipe,
            theme_window,
        })
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    ///
    /// `init` should always return an action of at least `TkAction::Reconfigure`.
    pub fn init<T>(&mut self, shared: &mut SharedState<T>) -> TkAction {
        debug!("Window::init");
        let mut mgr = self.mgr.manager(shared);
        mgr.send_action(TkAction::Reconfigure);

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => {
                    self.widget.trigger_callback(i, &mut mgr);
                }
                Callback::Close => (),
            }
        }

        mgr.unwrap_action()
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure<T>(&mut self, shared: &mut SharedState<T>) -> Option<Instant> {
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        debug!("Reconfiguring window (size = {:?})", size);

        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw_pipe) };
        self.widget.resize(&mut size_handle, size);
        self.mgr.configure(shared, &mut *self.widget);
        self.window.request_redraw();

        self.mgr.next_resume()
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        event: WindowEvent,
    ) -> (TkAction, Option<Instant>) {
        // Note: resize must be handled here to update self.swap_chain.
        let action = match event {
            WindowEvent::Resized(size) => self.do_resize(shared, size),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                // Note: API allows us to set new window size here.
                self.theme_window.set_dpi_factor(scale_factor as f32);
                self.mgr.set_dpi_factor(scale_factor);
                self.do_resize(shared, *new_inner_size)
            }
            event @ _ => self
                .mgr
                .manager(shared)
                .handle_winit(&mut *self.widget, event),
        };

        (action, self.mgr.next_resume())
    }

    pub fn handle_moved(&mut self) {
        self.mgr.region_moved(&mut *self.widget);
    }

    pub fn handle_closure<T>(mut self, shared: &mut SharedState<T>) -> TkAction {
        let mut mgr = self.mgr.manager(shared);

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => (),
                Callback::Close => {
                    self.widget.trigger_callback(i, &mut mgr);
                }
            }
        }
        if let Some(final_cb) = self.widget.final_callback() {
            final_cb(self.widget, &mut mgr);
        }

        mgr.unwrap_action()
    }

    pub fn update_timer<T>(&mut self, shared: &mut SharedState<T>) -> (TkAction, Option<Instant>) {
        let mut mgr = self.mgr.manager(shared);
        mgr.update_timer(&mut *self.widget);
        (mgr.unwrap_action(), self.mgr.next_resume())
    }

    pub fn update_handle<T>(
        &mut self,
        shared: &mut SharedState<T>,
        handle: UpdateHandle,
    ) -> TkAction {
        let mut mgr = self.mgr.manager(shared);
        mgr.update_handle(handle, &mut *self.widget);
        mgr.unwrap_action()
    }
}

// Internal functions
impl<TW: theme::Window<DrawPipe> + 'static> Window<TW> {
    fn do_resize<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
        size: PhysicalSize<u32>,
    ) -> TkAction {
        let size = size.into();
        if size == Size(self.sc_desc.width, self.sc_desc.height) {
            return TkAction::None;
        }

        debug!("Resizing window to size={:?}", size);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw_pipe) };
        self.widget.resize(&mut size_handle, size);

        let buf = self.draw_pipe.resize(&shared.device, size);
        shared.queue.submit(&[buf]);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);

        TkAction::Redraw
    }

    pub(crate) fn do_draw<T: theme::Theme<DrawPipe, Window = TW>>(
        &mut self,
        shared: &mut SharedState<T>,
    ) {
        trace!("Drawing window");
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let rect = Rect {
            pos: Coord::ZERO,
            size,
        };
        let frame = self.swap_chain.get_next_texture();
        let mut draw_handle = unsafe {
            shared
                .theme
                .draw_handle(&mut self.draw_pipe, &mut self.theme_window, rect)
        };
        self.widget
            .draw(&mut draw_handle, &self.mgr.manager(shared));
        let clear_color = to_wgpu_color(shared.theme.clear_colour());
        let buf = self
            .draw_pipe
            .render(&mut shared.device, &frame.view, clear_color);
        shared.queue.submit(&[buf]);
    }
}

fn to_wgpu_color(c: kas::draw::Colour) -> wgpu::Color {
    wgpu::Color {
        r: c.r as f64,
        g: c.g as f64,
        b: c.b as f64,
        a: c.a as f64,
    }
}
