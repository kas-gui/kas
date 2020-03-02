// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use log::{debug, info, trace};
use std::time::Instant;

use kas::event::{Callback, CursorIcon, ManagerState, UpdateHandle};
use kas::geom::{Coord, Rect, Size};
use kas::{ThemeAction, ThemeApi, TkAction, WindowId};
use kas_theme::Theme;
use winit::dpi::PhysicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;

use crate::draw::{CustomPipe, CustomWindow, DrawPipe, DrawWindow, TEX_FORMAT};
use crate::shared::{PendingAction, SharedState};
use crate::ProxyAction;

/// Per-window data
pub(crate) struct Window<CW: CustomWindow, TW> {
    widget: Box<dyn kas::Window>,
    mgr: ManagerState,
    /// The winit window
    pub(crate) window: winit::window::Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    draw: DrawWindow<CW>,
    theme_window: TW,
}

// Public functions, for use by the toolkit
impl<CW: CustomWindow + 'static, TW: kas_theme::Window<DrawWindow<CW>> + 'static> Window<CW, TW> {
    /// Construct a window
    pub fn new<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>, Window = TW>>(
        shared: &mut SharedState<C, T>,
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
            format: TEX_FORMAT,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mut draw = shared.draw.new_window(&mut shared.device, size);
        let theme_window = shared.theme.new_window(&mut draw, dpi_factor as f32);

        let mgr = ManagerState::new(dpi_factor);

        Ok(Window {
            widget,
            mgr,
            window,
            surface,
            sc_desc,
            swap_chain,
            draw,
            theme_window,
        })
    }

    /// Called by the `Toolkit` when the event loop starts to initialise
    /// windows. Optionally returns a callback time.
    ///
    /// `init` should always return an action of at least `TkAction::Reconfigure`.
    pub fn init<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>>>(
        &mut self,
        shared: &mut SharedState<C, T>,
    ) -> TkAction
    where
        T::Window: kas_theme::Window<DrawWindow<CW>>,
    {
        debug!("Window::init");
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        mgr.send_action(TkAction::Reconfigure);

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => {
                    self.widget.trigger_callback(i, &mut mgr);
                }
                Callback::Close => (),
            }
        }

        mgr.finish(&mut *self.widget)
    }

    /// Recompute layout of widgets and redraw
    pub fn reconfigure<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>>>(
        &mut self,
        shared: &mut SharedState<C, T>,
    ) -> Option<Instant>
    where
        T::Window: kas_theme::Window<DrawWindow<CW>>,
    {
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        debug!("Reconfiguring window (size = {:?})", size);

        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        let (min, max) = self.widget.resize(&mut size_handle, size);
        self.window.set_min_inner_size(min);
        self.window.set_max_inner_size(max);
        let mut tkw = TkWindow::new(&self.window, shared);
        self.mgr.configure(&mut tkw, &mut *self.widget);
        self.window.request_redraw();

        self.mgr.next_resume()
    }

    pub fn theme_resize<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>, Window = TW>>(
        &mut self,
        shared: &SharedState<C, T>,
    ) {
        debug!("Applying theme resize");
        let scale_factor = self.window.scale_factor() as f32;
        shared
            .theme
            .update_window(&mut self.theme_window, scale_factor);
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        let (min, max) = self.widget.resize(&mut size_handle, size);
        self.window.set_min_inner_size(min);
        self.window.set_max_inner_size(max);
        self.window.request_redraw();
    }

    /// Handle an event
    ///
    /// Return true to remove the window
    pub fn handle_event<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>, Window = TW>>(
        &mut self,
        shared: &mut SharedState<C, T>,
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
                shared
                    .theme
                    .update_window(&mut self.theme_window, scale_factor as f32);
                self.mgr.set_dpi_factor(scale_factor);
                self.do_resize(shared, *new_inner_size)
            }
            event @ _ => {
                let mut tkw = TkWindow::new(&self.window, shared);
                self.mgr
                    .manager(&mut tkw)
                    .handle_winit(&mut *self.widget, event)
            }
        };

        (action, self.mgr.next_resume())
    }

    pub fn handle_moved(&mut self) {
        self.mgr.region_moved(&mut *self.widget);
    }

    pub fn handle_closure<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>>>(
        mut self,
        shared: &mut SharedState<C, T>,
    ) -> TkAction
    where
        T::Window: kas_theme::Window<DrawWindow<CW>>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);

        for (i, condition) in self.widget.callbacks() {
            match condition {
                Callback::Start => (),
                Callback::Close => {
                    self.widget.trigger_callback(i, &mut mgr);
                }
            }
        }

        mgr.finish(&mut *self.widget)
    }

    pub fn update_timer<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>>>(
        &mut self,
        shared: &mut SharedState<C, T>,
    ) -> (TkAction, Option<Instant>)
    where
        T::Window: kas_theme::Window<DrawWindow<CW>>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        mgr.update_timer(&mut *self.widget);

        let action = mgr.finish(&mut *self.widget);
        (action, self.mgr.next_resume())
    }

    pub fn update_handle<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>>>(
        &mut self,
        shared: &mut SharedState<C, T>,
        handle: UpdateHandle,
        payload: u64,
    ) -> TkAction
    where
        T::Window: kas_theme::Window<DrawWindow<CW>>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        mgr.update_handle(&mut *self.widget, handle, payload);
        mgr.finish(&mut *self.widget)
    }
}

// Internal functions
impl<CW: CustomWindow + 'static, TW: kas_theme::Window<DrawWindow<CW>> + 'static> Window<CW, TW> {
    fn do_resize<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>, Window = TW>>(
        &mut self,
        shared: &mut SharedState<C, T>,
        size: PhysicalSize<u32>,
    ) -> TkAction {
        let size = size.into();
        if size == Size(self.sc_desc.width, self.sc_desc.height) {
            return TkAction::None;
        }

        debug!("Resizing window to size={:?}", size);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        self.widget.resize(&mut size_handle, size);
        drop(size_handle);

        let buf = self.draw.resize(&shared.device, size);
        shared.queue.submit(&[buf]);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);

        TkAction::Redraw
    }

    pub(crate) fn do_draw<C: CustomPipe<Window = CW>, T: Theme<DrawPipe<C>, Window = TW>>(
        &mut self,
        shared: &mut SharedState<C, T>,
    ) {
        trace!("Drawing window");
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let rect = Rect {
            pos: Coord::ZERO,
            size,
        };
        let mut draw_handle = unsafe {
            shared
                .theme
                .draw_handle(&mut self.draw, &mut self.theme_window, rect)
        };
        self.widget.draw(&mut draw_handle, &self.mgr);
        drop(draw_handle);

        let frame = self.swap_chain.get_next_texture();
        let clear_color = to_wgpu_color(shared.theme.clear_colour());
        let buf = shared.render(&mut self.draw, &frame.view, clear_color);
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

struct TkWindow<'a, C: CustomPipe, T> {
    window: &'a winit::window::Window,
    shared: &'a mut SharedState<C, T>,
}

impl<'a, C: CustomPipe, T> TkWindow<'a, C, T> {
    fn new(window: &'a winit::window::Window, shared: &'a mut SharedState<C, T>) -> Self {
        TkWindow { window, shared }
    }
}

impl<'a, C: CustomPipe, T: Theme<DrawPipe<C>>> kas::TkWindow for TkWindow<'a, C, T>
where
    T::Window: kas_theme::Window<DrawWindow<C::Window>>,
{
    fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId {
        // By far the simplest way to implement this is to let our call
        // anscestor, event::Loop::handle, do the work.
        //
        // In theory we could pass the EventLoopWindowTarget for *each* event
        // handled to create the winit window here or use statics to generate
        // errors now, but user code can't do much with this error anyway.
        let id = self.shared.next_window_id();
        self.shared
            .pending
            .push(PendingAction::AddWindow(id, widget));
        id
    }

    fn close_window(&mut self, id: WindowId) {
        self.shared.pending.push(PendingAction::CloseWindow(id));
    }

    fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.shared
            .pending
            .push(PendingAction::Update(handle, payload));
    }

    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        self.shared.get_clipboard()
    }

    #[inline]
    fn set_clipboard(&mut self, content: String) {
        self.shared.set_clipboard(content);
    }

    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeApi) -> ThemeAction) {
        match f(&mut self.shared.theme) {
            ThemeAction::None => (),
            ThemeAction::RedrawAll => self.shared.pending.push(PendingAction::RedrawAll),
            ThemeAction::ThemeResize => self.shared.pending.push(PendingAction::ThemeResize),
        }
    }

    #[inline]
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.window.set_cursor_icon(icon);
    }
}
