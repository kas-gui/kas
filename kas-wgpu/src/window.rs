// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use log::{debug, error, info, trace};
use std::rc::Rc;
use std::time::Instant;

use kas::cast::Cast;
use kas::draw::{SizeHandle, ThemeAction, ThemeApi};
use kas::event::{CursorIcon, ManagerState, UpdateHandle};
use kas::geom::{Coord, Rect, Size};
use kas::layout::SolveCache;
use kas::updatable::Updatable;
use kas::{TkAction, WindowId};
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
    pub(crate) window_id: WindowId,
    mgr: ManagerState,
    solve_cache: SolveCache,
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
    TW: kas_theme::Window + 'static,
{
    /// Construct a window
    pub fn new<C, T>(
        shared: &mut SharedState<C, T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        window_id: WindowId,
        mut widget: Box<dyn kas::Window>,
    ) -> Result<Self, OsError>
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let time = Instant::now();

        // Create draw immediately (with Size::ZERO) to find ideal window size
        let scale_factor = shared.scale_factor as f32;
        let mut draw = shared.draw.new_window(&mut shared.device, Size::ZERO);
        let mut theme_window = shared.theme.new_window(&mut draw, scale_factor);

        let mut size_handle = unsafe { theme_window.size_handle() };
        let solve_cache = SolveCache::find_constraints(widget.as_widget_mut(), &mut size_handle);
        // Opening a zero-size window causes a crash, so force at least 1x1:
        let ideal = solve_cache.ideal(true).max(Size(1, 1));
        drop(size_handle);

        let mut builder = WindowBuilder::new().with_inner_size(ideal);
        let restrict_dimensions = widget.restrict_dimensions();
        if restrict_dimensions.0 {
            builder = builder.with_min_inner_size(solve_cache.min(true));
        }
        if restrict_dimensions.1 {
            builder = builder.with_max_inner_size(ideal);
        }
        let window = builder.with_title(widget.title()).build(elwt)?;

        shared.init_clipboard(&window);

        let scale_factor = window.scale_factor();
        shared.scale_factor = scale_factor;
        let size: Size = window.inner_size().into();
        info!("Constucted new window with size {:?}", size);

        // draw was initially created with Size::ZERO; we must resize
        shared
            .draw
            .resize(&mut draw, &shared.device, &shared.queue, size);

        let surface = unsafe { shared.instance.create_surface(&window) };
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: TEX_FORMAT,
            width: size.0.cast(),
            height: size.1.cast(),
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = shared.device.create_swap_chain(&surface, &sc_desc);

        let mut mgr = ManagerState::new(shared.config.clone());
        let mut tkw = TkWindow::new(shared, &window, &mut theme_window);
        mgr.configure(&mut tkw, &mut *widget);

        let mut r = Window {
            widget,
            window_id,
            mgr,
            solve_cache,
            window,
            surface,
            sc_desc,
            swap_chain,
            draw,
            theme_window,
        };
        r.apply_size(shared);

        trace!("Window::new completed in {}µs", time.elapsed().as_micros());
        Ok(r)
    }

    pub fn theme_resize<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        debug!("Window::theme_resize");
        let scale_factor = self.window.scale_factor() as f32;
        shared
            .theme
            .update_window(&mut self.theme_window, scale_factor);
        self.solve_cache.invalidate_rule_cache();
        self.apply_size(shared);
    }

    /// Handle an event
    pub fn handle_event<C, T>(&mut self, shared: &mut SharedState<C, T>, event: WindowEvent)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        // Note: resize must be handled here to update self.swap_chain.
        match event {
            WindowEvent::Destroyed => (),
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
                self.solve_cache.invalidate_rule_cache();
                self.do_resize(shared, *new_inner_size);
            }
            event @ _ => {
                let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
                let widget = &mut *self.widget;
                self.mgr.with(&mut tkw, |mgr| {
                    mgr.handle_winit(widget, event);
                });
            }
        }
    }

    /// Update, after receiving all events
    pub fn update<C, T>(&mut self, shared: &mut SharedState<C, T>) -> (TkAction, Option<Instant>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        let action = self.mgr.update(&mut tkw, &mut *self.widget);
        drop(tkw);

        if action.contains(TkAction::CLOSE | TkAction::EXIT) {
            return (action, None);
        }
        if action.contains(TkAction::RECONFIGURE) {
            self.reconfigure(shared);
        }
        if action.contains(TkAction::RESIZE) {
            self.solve_cache.invalidate_rule_cache();
            self.apply_size(shared);
        } else if action.contains(TkAction::SET_SIZE) {
            self.apply_size(shared);
        }
        /*if action.contains(TkAction::Popup) {
            let widget = &mut self.widget;
            self.mgr.with(&mut tkw, |mgr| widget.resize_popups(mgr));
            self.mgr.region_moved(&mut tkw, &mut *self.widget);
            self.window.request_redraw();
        } else*/
        if action.contains(TkAction::REGION_MOVED) {
            let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
            self.mgr.region_moved(&mut tkw, &mut *self.widget);
            self.window.request_redraw();
        } else if action.contains(TkAction::REDRAW) {
            self.window.request_redraw();
        }

        (action, self.mgr.next_resume())
    }

    pub fn handle_closure<C, T>(mut self, shared: &mut SharedState<C, T>) -> TkAction
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        let widget = &mut *self.widget;
        self.mgr.with(&mut tkw, |mut mgr| {
            widget.handle_closure(&mut mgr);
        });
        self.mgr.update(&mut tkw, &mut *self.widget)
    }

    pub fn update_timer<C, T>(&mut self, shared: &mut SharedState<C, T>) -> Option<Instant>
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        let widget = &mut *self.widget;
        self.mgr.with(&mut tkw, |mgr| {
            mgr.update_timer(widget);
        });
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
        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        let widget = &mut *self.widget;
        self.mgr.with(&mut tkw, |mgr| {
            mgr.update_handle(widget, handle, payload);
        });
    }

    pub fn add_popup<C, T>(
        &mut self,
        shared: &mut SharedState<C, T>,
        id: WindowId,
        popup: kas::Popup,
    ) where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let window = &mut *self.widget;
        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        self.mgr.with(&mut tkw, |mut mgr| {
            kas::Window::add_popup(window, &mut mgr, id, popup);
        });
    }

    pub fn send_action(&mut self, action: TkAction) {
        self.mgr.send_action(action);
    }

    pub fn send_close<C, T>(&mut self, shared: &mut SharedState<C, T>, id: WindowId)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        if id == self.window_id {
            self.mgr.send_action(TkAction::CLOSE);
        } else {
            let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
            let widget = &mut *self.widget;
            self.mgr.with(&mut tkw, |mut mgr| {
                widget.remove_popup(&mut mgr, id);
            });
        }
    }
}

// Internal functions
impl<CW, TW> Window<CW, TW>
where
    CW: CustomWindow + 'static,
    TW: kas_theme::Window + 'static,
{
    /// Swap-chain size
    fn sc_size(&self) -> Size {
        Size::new(self.sc_desc.width.cast(), self.sc_desc.height.cast())
    }

    fn reconfigure<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let time = Instant::now();
        debug!("Window::reconfigure");

        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        self.mgr.configure(&mut tkw, &mut *self.widget);

        self.solve_cache.invalidate_rule_cache();
        self.apply_size(shared);
        trace!("reconfigure completed in {}µs", time.elapsed().as_micros());
    }

    fn apply_size<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let time = Instant::now();
        let rect = Rect::new(Coord::ZERO, self.sc_size());
        debug!("Resizing window to rect = {:?}", rect);

        let mut tkw = TkWindow::new(shared, &self.window, &mut self.theme_window);
        let solve_cache = &mut self.solve_cache;
        let widget = &mut self.widget;
        self.mgr.with(&mut tkw, |mgr| {
            solve_cache.apply_rect(widget.as_widget_mut(), mgr, rect, true);
            widget.resize_popups(mgr);
        });

        let restrict_dimensions = self.widget.restrict_dimensions();
        if restrict_dimensions.0 {
            self.window
                .set_min_inner_size(Some(self.solve_cache.min(true)));
        };
        if restrict_dimensions.1 {
            self.window
                .set_max_inner_size(Some(self.solve_cache.ideal(true)));
        };

        self.window.request_redraw();
        trace!("apply_size completed in {}µs", time.elapsed().as_micros());
    }

    fn do_resize<C, T>(&mut self, shared: &mut SharedState<C, T>, size: PhysicalSize<u32>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let time = Instant::now();
        let size = size.into();
        if size == self.sc_size() {
            return;
        }

        shared
            .draw
            .resize(&mut self.draw, &shared.device, &shared.queue, size);

        self.sc_desc.width = size.0.cast();
        self.sc_desc.height = size.1.cast();
        self.swap_chain = shared
            .device
            .create_swap_chain(&self.surface, &self.sc_desc);

        // Note that on resize, width adjustments may affect height
        // requirements; we therefore refresh size restrictions.
        self.apply_size(shared);

        trace!(
            "do_resize completed in {}µs (including apply_size time)",
            time.elapsed().as_micros()
        );
    }

    pub(crate) fn do_draw<C, T>(&mut self, shared: &mut SharedState<C, T>)
    where
        C: CustomPipe<Window = CW>,
        T: Theme<DrawPipe<C>, Window = TW>,
    {
        let time = Instant::now();
        let rect = Rect::new(Coord::ZERO, self.sc_size());

        unsafe {
            // Safety: we must drop draw_handle after draw call (wrong lifetime)
            let mut draw_handle =
                shared
                    .theme
                    .draw_handle(&mut self.draw, &mut self.theme_window, rect);
            self.widget.draw(&mut draw_handle, &self.mgr, false);
        }

        let time2 = Instant::now();
        let frame = match self.swap_chain.get_current_frame() {
            Ok(frame) => frame,
            Err(error) => {
                error!("Frame swap failed: {}", error);
                return;
            }
        };

        let time3 = Instant::now();
        // TODO: check frame.optimal ?
        let clear_color = to_wgpu_color(shared.theme.clear_color());
        shared.render(&mut self.draw, &frame.output.view, clear_color);

        let end = Instant::now();
        // Explanation: 'text' is the time to prepare positioned glyphs, 'frame-
        // swap' is mostly about sync, 'render' is time to feed the GPU.
        trace!(
            "do_draw completed in {}µs ({}µs text, {}µs frame-swap, {}µs render)",
            (end - time).as_micros(),
            self.draw.dur_text.as_micros(),
            (time3 - time2).as_micros(),
            (end - time3).as_micros()
        );
        self.draw.dur_text = Default::default();
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

struct TkWindow<'a, C: CustomPipe, T: Theme<DrawPipe<C>>>
where
    T::Window: kas_theme::Window,
{
    shared: &'a mut SharedState<C, T>,
    window: &'a winit::window::Window,
    theme_window: &'a mut T::Window,
}

impl<'a, C: CustomPipe, T: Theme<DrawPipe<C>>> TkWindow<'a, C, T>
where
    T::Window: kas_theme::Window,
{
    fn new(
        shared: &'a mut SharedState<C, T>,
        window: &'a winit::window::Window,
        theme_window: &'a mut T::Window,
    ) -> Self {
        TkWindow {
            shared,
            window,
            theme_window,
        }
    }
}

impl<'a, C, T> kas::ShellWindow for TkWindow<'a, C, T>
where
    C: CustomPipe,
    T: Theme<DrawPipe<C>>,
    T::Window: kas_theme::Window,
{
    fn add_popup(&mut self, popup: kas::Popup) -> WindowId {
        let id = self.shared.next_window_id();
        let parent_id = self.window.id();
        self.shared
            .pending
            .push(PendingAction::AddPopup(parent_id, id, popup));
        id
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

    fn update_shared_data(&mut self, handle: UpdateHandle, data: Rc<dyn Updatable>) {
        self.shared.update_shared_data(handle, data);
    }

    fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.shared.trigger_update(handle, payload);
    }

    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        self.shared.get_clipboard()
    }

    #[inline]
    fn set_clipboard<'c>(&mut self, content: String) {
        self.shared.set_clipboard(content);
    }

    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeApi) -> ThemeAction) {
        match f(&mut self.shared.theme) {
            ThemeAction::None => (),
            ThemeAction::RedrawAll => self.shared.pending.push(PendingAction::RedrawAll),
            ThemeAction::ThemeResize => self.shared.pending.push(PendingAction::ThemeResize),
        }
    }

    fn size_handle(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        use kas_theme::Window;
        let mut size_handle = unsafe { self.theme_window.size_handle() };
        f(&mut size_handle);
    }

    #[inline]
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.window.set_cursor_icon(icon);
    }
}
