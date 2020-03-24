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
use winit::window::WindowBuilder;

use crate::draw::{CustomPipe, CustomWindow, DrawPipe, DrawWindow, TEX_FORMAT};
use crate::shared::{PendingAction, SharedState};
use crate::ProxyAction;

/// Per-window data
pub(crate) struct Window<CW: CustomWindow, TW> {
    pub(crate) widget: Box<dyn kas::Window>,
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
impl<CW, TW> Window<CW, TW>
where
    CW: CustomWindow + 'static,
    TW: kas_theme::Window<DrawWindow<CW>> + 'static,
{
    /// Construct a window
    pub fn new<C, T>(
        shared: &mut SharedState<C, T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        mut widget: Box<dyn kas::Window>,
    ) -> Result<Self, OsError>
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        // Create draw immediately (with Size::ZERO) to find ideal window size
        let scale_factor = shared.scale_factor as f32;
        let mut draw = shared.draw.new_window(&mut shared.device, Size::ZERO);
        let mut theme_window = shared.theme.new_window(&mut draw, scale_factor);

        let mut size_handle = unsafe { theme_window.size_handle(&mut draw) };
        let (min, ideal) = widget.find_size(&mut size_handle);
        drop(size_handle);

        let mut builder = WindowBuilder::new().with_inner_size(ideal);
        if let Some(min) = min {
            builder = builder.with_min_inner_size(min);
        }
        let window = builder.with_title(widget.title()).build(elwt)?;

        let scale_factor = window.scale_factor();
        shared.scale_factor = scale_factor;
        let size: Size = window.inner_size().into();
        info!("Constucted new window with size {:?}", size);

        // draw was initially created with Size::ZERO; we must resize
        let buf = shared.draw.resize(&mut draw, &shared.device, size);
        shared.queue.submit(&[buf]);

        let surface = wgpu::Surface::create(&window);
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: TEX_FORMAT,
            width: size.0,
            height: size.1,
            present_mode: wgpu::PresentMode::Vsync,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mgr = ManagerState::new(scale_factor);

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
    pub fn init<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
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
    }

    /// Recompute layout of widgets and redraw
    fn reconfigure<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let rect = Rect::new(Coord::ZERO, size);
        debug!("Reconfiguring window (rect = {:?})", rect);

        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        let (min, max) = self.widget.resize(&mut size_handle, rect);
        self.window.set_min_inner_size(min);
        self.window.set_max_inner_size(max);
        let mut tkw = TkWindow::new(&self.window, shared);
        self.mgr.configure(&mut tkw, &mut *self.widget);
        self.window.request_redraw();
    }

    pub fn theme_resize<C, T>(&mut self, shared: &SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        debug!("Applying theme resize");
        let scale_factor = self.window.scale_factor() as f32;
        shared
            .theme
            .update_window(&mut self.theme_window, scale_factor);
        let size = Size(self.sc_desc.width, self.sc_desc.height);
        let rect = Rect::new(Coord::ZERO, size);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        let (min, max) = self.widget.resize(&mut size_handle, rect);
        self.window.set_min_inner_size(min);
        self.window.set_max_inner_size(max);
        self.window.request_redraw();
    }

    /// Handle an event
    pub fn handle_event<C, T>(&mut self, shared: &mut SharedState<C, T>, event: WindowEvent)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        // Note: resize must be handled here to update self.swap_chain.
        match event {
            WindowEvent::Resized(size) => self.do_resize(shared, size),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                // Note: API allows us to set new window size here.
                shared.scale_factor = scale_factor;
                shared
                    .theme
                    .update_window(&mut self.theme_window, scale_factor as f32);
                self.mgr.set_dpi_factor(scale_factor);
                self.do_resize(shared, *new_inner_size);
            }
            event @ _ => {
                let mut tkw = TkWindow::new(&self.window, shared);
                self.mgr
                    .manager(&mut tkw)
                    .handle_winit(&mut *self.widget, event);
            }
        }
    }

    /// Update, after receiving all events
    pub fn update<C, T>(&mut self, shared: &mut SharedState<C, T>) -> (TkAction, Option<Instant>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let action = self.mgr.manager(&mut tkw).finish(&mut *self.widget);

        match action {
            TkAction::None => (),
            TkAction::Redraw => self.window.request_redraw(),
            TkAction::RegionMoved => {
                self.mgr.region_moved(&mut *self.widget);
                self.window.request_redraw();
            }
            TkAction::Reconfigure => self.reconfigure(shared),
            TkAction::Close | TkAction::CloseAll => (),
        }

        (action, self.mgr.next_resume())
    }

    pub fn handle_closure<C, T>(mut self, shared: &mut SharedState<C, T>) -> TkAction
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
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

    pub fn update_timer<C, T>(&mut self, shared: &mut SharedState<C, T>) -> Option<Instant>
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        mgr.update_timer(&mut *self.widget);
        self.mgr.next_resume()
    }

    pub fn update_handle<C, T>(
        &mut self,
        shared: &mut SharedState<C, T>,
        handle: UpdateHandle,
        payload: u64,
    ) where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        mgr.update_handle(&mut *self.widget, handle, payload);
    }

    pub fn add_popup<C, T>(&mut self, shared: &mut SharedState<C, T>, popup: kas::Popup)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let window = &mut *self.widget;
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        let mut tkw = TkWindow::new(&self.window, shared);
        let mut mgr = self.mgr.manager(&mut tkw);
        kas::Window::add_popup(window, &mut size_handle, &mut mgr, popup);
    }

    pub fn send_action(&mut self, action: TkAction) {
        self.mgr.send_action(action);
    }
}

// Internal functions
impl<CW, TW> Window<CW, TW>
where
    CW: CustomWindow + 'static,
    TW: kas_theme::Window<DrawWindow<CW>> + 'static,
{
    fn do_resize<C, T>(&mut self, shared: &mut SharedState<C, T>, size: PhysicalSize<u32>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let size = size.into();
        if size == Size(self.sc_desc.width, self.sc_desc.height) {
            return;
        }

        let rect = Rect::new(Coord::ZERO, size);
        debug!("Resizing window to rect = {:?}", rect);
        let mut size_handle = unsafe { self.theme_window.size_handle(&mut self.draw) };
        self.widget.resize(&mut size_handle, rect);
        drop(size_handle);

        let buf = shared.draw.resize(&mut self.draw, &shared.device, size);
        shared.queue.submit(&[buf]);

        self.sc_desc.width = size.0;
        self.sc_desc.height = size.1;
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);

        self.window.request_redraw();
    }

    pub(crate) fn do_draw<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
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
        shared.render(&mut self.draw, &frame.view, clear_color);
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

impl<'a, C, T> kas::TkWindow for TkWindow<'a, C, T>
where
    C: CustomPipe,
    T: Theme<DrawPipe<C>>,
    T::Window: kas_theme::Window<DrawWindow<C::Window>>,
{
    fn add_popup(&mut self, popup: kas::Popup) {
        let id = self.window.id();
        self.shared.pending.push(PendingAction::AddPopup(id, popup));
    }

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
    fn get_clipboard(&mut self) -> Option<kas::CowString> {
        self.shared.get_clipboard()
    }

    #[inline]
    fn set_clipboard<'c>(&mut self, content: kas::CowStringL<'c>) {
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
