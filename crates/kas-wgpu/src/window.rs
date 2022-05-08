// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `Window` and `WindowList` types

use log::{debug, error, info, trace};
use std::time::Instant;

use kas::cast::Cast;
use kas::draw::{AnimationState, DrawIface, DrawShared, PassId};
use kas::event::{CursorIcon, EventState, UpdateHandle};
use kas::geom::{Coord, Rect, Size};
use kas::layout::{SetRectMgr, SolveCache};
use kas::theme::{DrawMgr, SizeHandle, SizeMgr, ThemeControl};
use kas::{TkAction, WidgetExt, WindowId};
use kas_theme::{Theme, Window as _};
use winit::dpi::PhysicalSize;
use winit::error::OsError;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use crate::draw::{CustomPipe, DrawPipe, DrawWindow};
use crate::shared::{PendingAction, SharedState};
use crate::ProxyAction;

/// Per-window data
pub(crate) struct Window<C: CustomPipe, T: Theme<DrawPipe<C>>> {
    pub(crate) widget: Box<dyn kas::Window>,
    pub(crate) window_id: WindowId,
    ev_state: EventState,
    solve_cache: SolveCache,
    /// The winit window
    pub(crate) window: winit::window::Window,
    surface: wgpu::Surface,
    sc_desc: wgpu::SurfaceConfiguration,
    draw: DrawWindow<C::Window>,
    theme_window: T::Window,
    next_avail_frame_time: Instant,
    queued_frame_time: Option<Instant>,
}

// Public functions, for use by the toolkit
impl<C: CustomPipe, T: Theme<DrawPipe<C>>> Window<C, T> {
    /// Construct a window
    pub fn new(
        shared: &mut SharedState<C, T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        window_id: WindowId,
        mut widget: Box<dyn kas::Window>,
    ) -> Result<Self, OsError> {
        let time = Instant::now();

        // Wayland only supports windows constructed via logical size
        #[allow(unused_assignments, unused_mut)]
        let mut use_logical_size = false;
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            use winit::platform::unix::EventLoopWindowTargetExtUnix;
            use_logical_size = elwt.is_wayland();
        }

        let scale_factor = if use_logical_size {
            1.0
        } else {
            shared.scale_factor as f32
        };

        let mut theme_window = shared.theme.new_window(scale_factor);

        let mut ev_state = EventState::new(shared.config.clone(), scale_factor);
        let mut tkw = TkWindow::new(shared, None, &mut theme_window);
        ev_state.full_configure(&mut tkw, widget.as_widget_mut());

        let size_mgr = SizeMgr::new(theme_window.size_handle());
        let mut solve_cache = SolveCache::find_constraints(widget.as_widget_mut(), size_mgr);

        // Opening a zero-size window causes a crash, so force at least 1x1:
        let ideal = solve_cache.ideal(true).max(Size(1, 1));
        let ideal = match use_logical_size {
            false => ideal.as_physical(),
            true => ideal.as_logical(),
        };

        let mut builder = WindowBuilder::new().with_inner_size(ideal);
        let restrict_dimensions = widget.restrict_dimensions();
        if restrict_dimensions.0 {
            let min = solve_cache.min(true);
            let min = match use_logical_size {
                false => min.as_physical(),
                true => min.as_logical(),
            };
            builder = builder.with_min_inner_size(min);
        }
        if restrict_dimensions.1 {
            builder = builder.with_max_inner_size(ideal);
        }
        let window = builder
            .with_title(widget.title())
            .with_window_icon(widget.icon())
            .build(elwt)?;

        shared.init_clipboard(&window);

        let scale_factor = window.scale_factor();
        shared.scale_factor = scale_factor;
        let size: Size = window.inner_size().cast();
        info!(
            "Constucted new window with physical size {:?}, scale factor {}",
            size, scale_factor
        );

        // Now that we have a scale factor, we may need to resize:
        if use_logical_size && scale_factor != 1.0 {
            let scale_factor = scale_factor as f32;
            ev_state.set_scale_factor(scale_factor);
            shared.theme.update_window(&mut theme_window, scale_factor);
            solve_cache.invalidate_rule_cache();
        }

        let mut draw = shared.draw.draw.new_window();
        shared.draw.draw.resize(&mut draw, size);

        let surface = unsafe { shared.instance.create_surface(&window) };
        let sc_desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: crate::draw::RENDER_TEX_FORMAT,
            width: size.0.cast(),
            height: size.1.cast(),
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&shared.draw.draw.device, &sc_desc);

        let mut r = Window {
            widget,
            window_id,
            ev_state,
            solve_cache,
            window,
            surface,
            sc_desc,
            draw,
            theme_window,
            next_avail_frame_time: time,
            queued_frame_time: Some(time),
        };
        r.apply_size(shared, true);

        trace!("Window::new completed in {}µs", time.elapsed().as_micros());
        Ok(r)
    }

    /// Handle an event
    pub fn handle_event(&mut self, shared: &mut SharedState<C, T>, event: WindowEvent) {
        // Note: resize must be handled here to re-configure self.surface.
        match event {
            WindowEvent::Destroyed => (),
            WindowEvent::Resized(size) => self.do_resize(shared, size),
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                // Note: API allows us to set new window size here.
                shared.scale_factor = scale_factor;
                let scale_factor = scale_factor as f32;
                self.ev_state.set_scale_factor(scale_factor);
                shared
                    .theme
                    .update_window(&mut self.theme_window, scale_factor);
                self.solve_cache.invalidate_rule_cache();
                self.do_resize(shared, *new_inner_size);
            }
            event => {
                let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
                let widget = &mut *self.widget;
                self.ev_state.with(&mut tkw, |mgr| {
                    mgr.handle_winit(widget.as_widget_mut(), event);
                });

                if self.ev_state.action.contains(TkAction::RECONFIGURE) {
                    // Reconfigure must happen before further event handling
                    self.reconfigure(shared);
                    self.ev_state.action.remove(TkAction::RECONFIGURE);
                }
            }
        }
    }

    /// Update, after receiving all events
    pub fn update(&mut self, shared: &mut SharedState<C, T>) -> (TkAction, Option<Instant>) {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let action = self.ev_state.update(&mut tkw, self.widget.as_widget_mut());
        drop(tkw);

        if action.contains(TkAction::CLOSE | TkAction::EXIT) {
            return (action, None);
        }
        self.handle_action(shared, action);

        let mut resume = self.ev_state.next_resume();

        if let Some(time) = self.queued_frame_time {
            if time <= Instant::now() {
                self.window.request_redraw();
                self.queued_frame_time = None;
            } else {
                resume = resume.map(|t| t.min(time)).or(Some(time));
            }
        }

        (action, resume)
    }

    /// Handle an action (excludes handling of CLOSE and EXIT)
    pub fn handle_action(&mut self, shared: &mut SharedState<C, T>, action: TkAction) {
        if action.contains(TkAction::RECONFIGURE) {
            self.reconfigure(shared);
        }
        if action.contains(TkAction::THEME_UPDATE) {
            let scale_factor = self.window.scale_factor() as f32;
            shared
                .theme
                .update_window(&mut self.theme_window, scale_factor);
        }
        if action.contains(TkAction::RESIZE) {
            self.solve_cache.invalidate_rule_cache();
            self.apply_size(shared, false);
        } else if action.contains(TkAction::SET_SIZE) {
            self.apply_size(shared, false);
        }
        /*if action.contains(TkAction::Popup) {
            let widget = &mut self.widget;
            self.ev_state.with(&mut tkw, |mgr| widget.resize_popups(mgr));
            self.ev_state.region_moved(&mut tkw, &mut *self.widget);
        } else*/
        if action.contains(TkAction::REGION_MOVED) {
            let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
            self.ev_state
                .region_moved(&mut tkw, self.widget.as_widget_mut());
        }
        if !action.is_empty() {
            self.queued_frame_time = Some(self.next_avail_frame_time);
        }
    }

    pub fn handle_closure(mut self, shared: &mut SharedState<C, T>) -> TkAction {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = &mut *self.widget;
        self.ev_state.with(&mut tkw, |mgr| {
            widget.handle_closure(mgr);
        });
        self.ev_state.update(&mut tkw, self.widget.as_widget_mut())
    }

    pub fn update_timer(&mut self, shared: &mut SharedState<C, T>) -> Option<Instant> {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = &mut *self.widget;
        self.ev_state.with(&mut tkw, |mgr| {
            mgr.update_timer(widget.as_widget_mut());
        });
        self.next_resume()
    }

    pub fn update_handle(
        &mut self,
        shared: &mut SharedState<C, T>,
        handle: UpdateHandle,
        payload: u64,
    ) {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = &mut *self.widget;
        self.ev_state.with(&mut tkw, |mgr| {
            mgr.update_handle(widget.as_widget_mut(), handle, payload);
        });
    }

    pub fn add_popup(&mut self, shared: &mut SharedState<C, T>, id: WindowId, popup: kas::Popup) {
        let window = &mut *self.widget;
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        self.ev_state.with(&mut tkw, |mgr| {
            kas::Window::add_popup(window, mgr, id, popup);
        });
    }

    pub fn send_action(&mut self, action: TkAction) {
        self.ev_state.send_action(action);
    }

    pub fn send_close(&mut self, shared: &mut SharedState<C, T>, id: WindowId) {
        if id == self.window_id {
            self.ev_state.send_action(TkAction::CLOSE);
        } else {
            let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
            let widget = &mut *self.widget;
            self.ev_state.with(&mut tkw, |mgr| {
                widget.remove_popup(mgr, id);
            });
        }
    }
}

// Internal functions
impl<C: CustomPipe, T: Theme<DrawPipe<C>>> Window<C, T> {
    /// Swap-chain size
    fn sc_size(&self) -> Size {
        Size::new(self.sc_desc.width.cast(), self.sc_desc.height.cast())
    }

    fn reconfigure(&mut self, shared: &mut SharedState<C, T>) {
        let time = Instant::now();
        debug!("Window::reconfigure");

        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        self.ev_state
            .full_configure(&mut tkw, self.widget.as_widget_mut());

        self.solve_cache.invalidate_rule_cache();
        self.apply_size(shared, false);
        trace!("reconfigure completed in {}µs", time.elapsed().as_micros());
    }

    fn apply_size(&mut self, shared: &mut SharedState<C, T>, first: bool) {
        let time = Instant::now();
        let rect = Rect::new(Coord::ZERO, self.sc_size());
        debug!("Resizing window to rect = {:?}", rect);

        let solve_cache = &mut self.solve_cache;
        let widget = &mut self.widget;
        let mut mgr = SetRectMgr::new(
            self.theme_window.size_handle(),
            &mut shared.draw,
            &mut self.ev_state,
        );
        solve_cache.apply_rect(widget.as_widget_mut(), &mut mgr, rect, true, first);
        widget.resize_popups(&mut mgr);

        let restrict_dimensions = self.widget.restrict_dimensions();
        if restrict_dimensions.0 {
            let min = self.solve_cache.min(true).as_physical();
            self.window.set_min_inner_size(Some(min));
        };
        if restrict_dimensions.1 {
            let ideal = self.solve_cache.ideal(true).as_physical();
            self.window.set_max_inner_size(Some(ideal));
        };

        self.window.request_redraw();
        trace!("apply_size completed in {}µs", time.elapsed().as_micros());
    }

    fn do_resize(&mut self, shared: &mut SharedState<C, T>, size: PhysicalSize<u32>) {
        let time = Instant::now();
        let size = size.cast();
        if size == self.sc_size() {
            return;
        }

        shared.draw.draw.resize(&mut self.draw, size);

        self.sc_desc.width = size.0.cast();
        self.sc_desc.height = size.1.cast();
        self.surface
            .configure(&shared.draw.draw.device, &self.sc_desc);

        // Note that on resize, width adjustments may affect height
        // requirements; we therefore refresh size restrictions.
        self.apply_size(shared, false);

        trace!(
            "do_resize completed in {}µs (including apply_size time)",
            time.elapsed().as_micros()
        );
    }

    // Draw. Return true when further event processing is needed immediately.
    pub(crate) fn do_draw(&mut self, shared: &mut SharedState<C, T>) -> bool {
        let start = Instant::now();
        self.next_avail_frame_time = start + shared.frame_dur;

        {
            let draw = DrawIface {
                draw: &mut self.draw,
                shared: &mut shared.draw,
                pass: PassId::new(0),
            };

            #[cfg(not(feature = "gat"))]
            unsafe {
                // Safety: lifetimes do not escape the returned draw_handle value.
                let mut draw_handle =
                    shared
                        .theme
                        .draw_handle(draw, &mut self.ev_state, &mut self.theme_window);
                let draw_mgr = DrawMgr::new(&mut draw_handle, self.widget.id());
                self.widget.draw(draw_mgr);
            }
            #[cfg(feature = "gat")]
            {
                let mut draw_handle =
                    shared
                        .theme
                        .draw_handle(draw, &mut self.ev_state, &mut self.theme_window);
                let draw_mgr = DrawMgr::new(&mut draw_handle, self.widget.id());
                self.widget.draw(draw_mgr);
            }
        }

        self.queued_frame_time = match self.draw.animation {
            AnimationState::None => None,
            AnimationState::Animate => Some(self.next_avail_frame_time),
            AnimationState::Timed(time) => Some(time.max(self.next_avail_frame_time)),
        };
        self.draw.animation = AnimationState::None;
        self.ev_state.action -= TkAction::REDRAW; // we just drew
        if !self.ev_state.action.is_empty() {
            info!("do_draw: abort and enqueue `Self::update` due to non-empty actions");
            return true;
        }

        let time2 = Instant::now();
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                error!("Failed to get frame texture: {}", e);
                // It may be possible to recover by calling surface.configure(...) then retrying
                // surface.get_current_texture(), but is doing so ever useful?
                return true;
            }
        };
        // TODO: check frame.suboptimal ?
        let view = frame.texture.create_view(&Default::default());

        let clear_color = to_wgpu_color(shared.theme.clear_color());
        shared.render(&mut self.draw, &view, clear_color);

        frame.present();

        let end = Instant::now();
        // Explanation: 'text' is the time to prepare positioned glyphs, 'frame-
        // swap' is mostly about sync, 'render' is time to feed the GPU.
        trace!(
            "do_draw completed in {}µs ({}μs widgets, {}µs text, {}µs render)",
            (end - start).as_micros(),
            (time2 - start).as_micros(),
            self.draw.text.dur_micros(),
            (end - time2).as_micros()
        );
        false
    }

    pub(crate) fn next_resume(&self) -> Option<Instant> {
        match (self.ev_state.next_resume(), self.queued_frame_time) {
            (Some(t1), Some(t2)) => Some(t1.min(t2)),
            (Some(t), None) => Some(t),
            (None, Some(t)) => Some(t),
            (None, None) => None,
        }
    }
}

fn to_wgpu_color(c: kas::draw::color::Rgba) -> wgpu::Color {
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
    window: Option<&'a winit::window::Window>,
    theme_window: &'a mut T::Window,
}

impl<'a, C: CustomPipe, T: Theme<DrawPipe<C>>> TkWindow<'a, C, T>
where
    T::Window: kas_theme::Window,
{
    fn new(
        shared: &'a mut SharedState<C, T>,
        window: Option<&'a winit::window::Window>,
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
    fn add_popup(&mut self, popup: kas::Popup) -> Option<WindowId> {
        self.window.map(|w| w.id()).map(|parent_id| {
            let id = self.shared.next_window_id();
            self.shared
                .pending
                .push(PendingAction::AddPopup(parent_id, id, popup));
            id
        })
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

    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeControl) -> TkAction) {
        let action = f(&mut self.shared.theme);
        self.shared.pending.push(PendingAction::TkAction(action));
    }

    fn size_and_draw_shared(
        &mut self,
        f: &mut dyn FnMut(&mut dyn SizeHandle, &mut dyn DrawShared),
    ) {
        use kas_theme::Window;
        let mut size_handle = self.theme_window.size_handle();
        f(&mut size_handle, &mut self.shared.draw);
    }

    #[inline]
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        if let Some(window) = self.window {
            window.set_cursor_icon(icon);
        }
    }
}
