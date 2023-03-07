// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window types

use super::{PendingAction, Platform, ProxyAction, SharedState, ShellWindow, WindowSurface};
use kas::cast::Cast;
use kas::draw::{color::Rgba, AnimationState, DrawShared};
use kas::event::{ConfigMgr, CursorIcon, EventState, UpdateId};
use kas::geom::{Coord, Rect, Size};
use kas::layout::SolveCache;
use kas::theme::{DrawMgr, SizeMgr, ThemeControl, ThemeSize};
use kas::theme::{Theme, Window as _};
#[cfg(all(wayland_platform, feature = "clipboard"))]
use kas::util::warn_about_error;
use kas::{Action, Layout, WidgetCore, WidgetExt, Window as _, WindowId};
use std::mem::take;
use std::time::Instant;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

#[crate::autoimpl(Deref, DerefMut using self.window)]
pub(super) struct WindowData {
    window: winit::window::Window,
    #[cfg(all(wayland_platform, feature = "clipboard"))]
    wayland_clipboard: Option<smithay_clipboard::Clipboard>,
}

impl WindowData {
    #[cfg(not(all(wayland_platform, feature = "clipboard")))]
    fn new(window: winit::window::Window) -> Self {
        WindowData { window }
    }

    #[cfg(all(wayland_platform, feature = "clipboard"))]
    fn new(window: winit::window::Window) -> Self {
        use winit::platform::wayland::WindowExtWayland;
        let wayland_clipboard = window
            .wayland_display()
            .map(|display| unsafe { smithay_clipboard::Clipboard::new(display) });
        WindowData {
            window,
            wayland_clipboard,
        }
    }
}

/// Per-window data
pub struct Window<S: WindowSurface, T: Theme<S::Shared>> {
    pub(super) widget: kas::RootWidget,
    pub(super) window_id: WindowId,
    ev_state: EventState,
    solve_cache: SolveCache,
    pub(super) window: WindowData,
    theme_window: T::Window,
    next_avail_frame_time: Instant,
    queued_frame_time: Option<Instant>,
    surface: S,
}

// Public functions, for use by the toolkit
impl<S: WindowSurface, T: Theme<S::Shared>> Window<S, T> {
    /// Construct a window
    pub(super) fn new(
        shared: &mut SharedState<S, T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
        window_id: WindowId,
        widget: Box<dyn kas::Window>,
    ) -> super::Result<Self> {
        let time = Instant::now();

        let mut widget = kas::RootWidget::new(widget);

        // Wayland only supports windows constructed via logical size
        let use_logical_size = shared.platform.is_wayland();

        let scale_factor = if use_logical_size {
            1.0
        } else {
            shared.scale_factor as f32
        };

        let mut theme_window = shared.theme.new_window(scale_factor);
        let dpem = theme_window.size().dpem();

        let mut ev_state = EventState::new(shared.config.clone(), scale_factor, dpem);
        let mut tkw = TkWindow::new(shared, None, &mut theme_window);
        ev_state.full_configure(&mut tkw, widget.as_widget_mut());

        let size_mgr = SizeMgr::new(theme_window.size());
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
            .with_decorations(widget.decorations() == kas::Decorations::Server)
            .with_transparent(widget.transparent())
            .build(elwt)?;

        let scale_factor = window.scale_factor();
        shared.scale_factor = scale_factor;
        let size: Size = window.inner_size().cast();
        log::info!(
            "new: constructed with physical size {:?}, scale factor {}",
            size,
            scale_factor
        );

        // Now that we have a scale factor, we may need to resize:
        if use_logical_size && scale_factor != 1.0 {
            let scale_factor = scale_factor as f32;
            shared.theme.update_window(&mut theme_window, scale_factor);
            let dpem = theme_window.size().dpem();
            ev_state.set_scale_factor(scale_factor, dpem);
            solve_cache.invalidate_rule_cache();
        }

        let surface = S::new(&mut shared.draw.draw, size, &window)?;

        let mut r = Window {
            widget,
            window_id,
            ev_state,
            solve_cache,
            window: WindowData::new(window),
            theme_window,
            next_avail_frame_time: time,
            queued_frame_time: Some(time),
            surface,
        };
        r.apply_size(shared, true);

        log::trace!(target: "kas_perf::wgpu::window", "new: {}µs", time.elapsed().as_micros());
        Ok(r)
    }

    /// Handle an event
    pub(super) fn handle_event(&mut self, shared: &mut SharedState<S, T>, event: WindowEvent) {
        // Note: resize must be handled here to re-configure self.surface.
        match event {
            WindowEvent::Destroyed => (),
            WindowEvent::Resized(size) => {
                if self.surface.do_resize(&mut shared.draw.draw, size.cast()) {
                    self.apply_size(shared, false);
                }
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                // Note: API allows us to set new window size here.
                shared.scale_factor = scale_factor;
                let scale_factor = scale_factor as f32;
                shared
                    .theme
                    .update_window(&mut self.theme_window, scale_factor);
                let dpem = self.theme_window.size().dpem();
                self.ev_state.set_scale_factor(scale_factor, dpem);
                self.solve_cache.invalidate_rule_cache();
                let size = (*new_inner_size).cast();
                if self.surface.do_resize(&mut shared.draw.draw, size) {
                    self.apply_size(shared, false);
                }
            }
            event => {
                let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
                self.ev_state.with(&mut tkw, |mgr| {
                    mgr.handle_winit(&mut self.widget, event);
                });

                if self.ev_state.action.contains(Action::RECONFIGURE) {
                    // Reconfigure must happen before further event handling
                    self.reconfigure(shared);
                    self.ev_state.action.remove(Action::RECONFIGURE);
                }
            }
        }
    }

    /// Update, after receiving all events
    pub(super) fn update(&mut self, shared: &mut SharedState<S, T>) -> (Action, Option<Instant>) {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let action = self.ev_state.update(&mut tkw, self.widget.as_widget_mut());

        if action.contains(Action::CLOSE | Action::EXIT) {
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

    /// Post-draw updates
    ///
    /// Returns: time of next scheduled resume.
    pub(super) fn post_draw(&mut self, shared: &mut SharedState<S, T>) -> Option<Instant> {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let has_action = self
            .ev_state
            .post_draw(&mut tkw, self.widget.as_widget_mut());

        if has_action {
            self.queued_frame_time = Some(self.next_avail_frame_time);
        }

        self.next_resume()
    }

    /// Handle an action (excludes handling of CLOSE and EXIT)
    pub(super) fn handle_action(&mut self, shared: &mut SharedState<S, T>, action: Action) {
        if action.contains(Action::RECONFIGURE) {
            self.reconfigure(shared);
        }
        if action.contains(Action::THEME_UPDATE) {
            let scale_factor = self.window.scale_factor() as f32;
            shared
                .theme
                .update_window(&mut self.theme_window, scale_factor);
        }
        if action.contains(Action::RESIZE) {
            self.solve_cache.invalidate_rule_cache();
            self.apply_size(shared, false);
        } else if action.contains(Action::SET_SIZE) {
            self.apply_size(shared, false);
        }
        /*if action.contains(Action::Popup) {
            let widget = &mut self.widget;
            self.ev_state.with(&mut tkw, |mgr| widget.resize_popups(mgr));
            self.ev_state.region_moved(&mut *self.widget);
        } else*/
        if action.contains(Action::REGION_MOVED) {
            self.ev_state.region_moved(&mut self.widget.as_widget_mut());
        }
        if !action.is_empty() {
            self.queued_frame_time = Some(self.next_avail_frame_time);
        }
    }

    pub(super) fn handle_closure(mut self, shared: &mut SharedState<S, T>) -> Action {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = &mut self.widget;
        self.ev_state.with(&mut tkw, |mgr| {
            widget.handle_closure(mgr);
        });
        self.ev_state.update(&mut tkw, self.widget.as_widget_mut())
    }

    pub(super) fn update_timer(&mut self, shared: &mut SharedState<S, T>) -> Option<Instant> {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = self.widget.as_widget_mut();
        self.ev_state.with(&mut tkw, |mgr| mgr.update_timer(widget));
        self.next_resume()
    }

    pub(super) fn update_widgets(
        &mut self,
        shared: &mut SharedState<S, T>,
        id: UpdateId,
        payload: u64,
    ) {
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        let widget = self.widget.as_widget_mut();
        self.ev_state
            .with(&mut tkw, |mgr| mgr.update_widgets(widget, id, payload));
    }

    pub(super) fn add_popup(
        &mut self,
        shared: &mut SharedState<S, T>,
        id: WindowId,
        popup: kas::Popup,
    ) {
        let widget = &mut self.widget;
        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        self.ev_state
            .with(&mut tkw, |mgr| widget.add_popup(mgr, id, popup));
    }

    pub(super) fn send_action(&mut self, action: Action) {
        self.ev_state.send_action(action);
    }

    pub(super) fn send_close(&mut self, shared: &mut SharedState<S, T>, id: WindowId) {
        if id == self.window_id {
            self.ev_state.send_action(Action::CLOSE);
        } else {
            let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
            let widget = &mut self.widget;
            self.ev_state
                .with(&mut tkw, |mgr| widget.remove_popup(mgr, id));
        }
    }
}

// Internal functions
impl<S: WindowSurface, T: Theme<S::Shared>> Window<S, T> {
    fn reconfigure(&mut self, shared: &mut SharedState<S, T>) {
        let time = Instant::now();
        log::debug!("reconfigure");

        let mut tkw = TkWindow::new(shared, Some(&self.window), &mut self.theme_window);
        self.ev_state
            .full_configure(&mut tkw, self.widget.as_widget_mut());

        self.solve_cache.invalidate_rule_cache();
        self.apply_size(shared, false);
        log::trace!(target: "kas_perf::wgpu::window", "reconfigure: {}µs", time.elapsed().as_micros());
    }

    fn apply_size(&mut self, shared: &mut SharedState<S, T>, first: bool) {
        let time = Instant::now();
        let rect = Rect::new(Coord::ZERO, self.surface.size());
        log::debug!("apply_size: rect={rect:?}");

        let solve_cache = &mut self.solve_cache;
        let widget = &mut self.widget;
        let mut mgr = ConfigMgr::new(
            self.theme_window.size(),
            &mut shared.draw,
            &mut self.ev_state,
        );
        solve_cache.apply_rect(widget.as_widget_mut(), &mut mgr, rect, true);
        if first {
            solve_cache.print_widget_heirarchy(widget.as_widget_mut());
        }
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
        log::trace!(
            target: "kas_perf::wgpu::window",
            "apply_size: {}µs", time.elapsed().as_micros(),
        );
    }

    /// Draw
    ///
    /// Returns an error when drawing is aborted and further event handling may
    /// be needed before a redraw.
    pub(super) fn do_draw(&mut self, shared: &mut SharedState<S, T>) -> Result<(), ()> {
        let start = Instant::now();
        self.next_avail_frame_time = start + self.ev_state.config().frame_dur();

        {
            let draw = self.surface.draw_iface(&mut shared.draw);

            let mut draw = shared
                .theme
                .draw(draw, &mut self.ev_state, &mut self.theme_window);
            let draw_mgr = DrawMgr::new(&mut draw, self.widget.id());
            self.widget.draw(draw_mgr);
        }
        let time2 = Instant::now();

        let anim = take(&mut self.surface.common_mut().anim);
        self.queued_frame_time = match anim {
            AnimationState::None => None,
            AnimationState::Animate => Some(self.next_avail_frame_time),
            AnimationState::Timed(time) => Some(time.max(self.next_avail_frame_time)),
        };
        self.ev_state.action -= Action::REDRAW; // we just drew
        if !self.ev_state.action.is_empty() {
            log::info!("do_draw: abort and enqueue `Self::update` due to non-empty actions");
            return Err(());
        }

        let clear_color = if self.widget.transparent() {
            Rgba::TRANSPARENT
        } else {
            shared.theme.clear_color()
        };
        self.surface.present(&mut shared.draw.draw, clear_color);

        let text_dur_micros = take(&mut self.surface.common_mut().dur_text);
        let end = Instant::now();
        log::trace!(
            target: "kas_perf::wgpu::window",
            "do_draw: {}µs ({}μs widgets, {}µs text, {}µs render)",
            (end - start).as_micros(),
            (time2 - start).as_micros(),
            text_dur_micros.as_micros(),
            (end - time2).as_micros()
        );
        Ok(())
    }

    fn next_resume(&self) -> Option<Instant> {
        match (self.ev_state.next_resume(), self.queued_frame_time) {
            (Some(t1), Some(t2)) => Some(t1.min(t2)),
            (Some(t), None) => Some(t),
            (None, Some(t)) => Some(t),
            (None, None) => None,
        }
    }
}

struct TkWindow<'a, S: WindowSurface, T: Theme<S::Shared>>
where
    T::Window: kas::theme::Window,
{
    shared: &'a mut SharedState<S, T>,
    window: Option<&'a WindowData>,
    theme_window: &'a mut T::Window,
}

impl<'a, S: WindowSurface, T: Theme<S::Shared>> TkWindow<'a, S, T>
where
    T::Window: kas::theme::Window,
{
    fn new(
        shared: &'a mut SharedState<S, T>,
        window: Option<&'a WindowData>,
        theme_window: &'a mut T::Window,
    ) -> Self {
        TkWindow {
            shared,
            window,
            theme_window,
        }
    }
}

impl<'a, S, T> ShellWindow for TkWindow<'a, S, T>
where
    S: WindowSurface,
    T: Theme<S::Shared>,
    T::Window: kas::theme::Window,
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

    fn update_all(&mut self, id: UpdateId, payload: u64) {
        self.shared.update_all(id, payload);
    }

    fn drag_window(&self) {
        if let Some(window) = self.window {
            let _result = window.drag_window();
        }
    }

    #[inline]
    fn get_clipboard(&mut self) -> Option<String> {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self
            .window
            .as_ref()
            .and_then(|data| data.wayland_clipboard.as_ref())
        {
            return match cb.load() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn_about_error("Failed to get clipboard contents", &e);
                    None
                }
            };
        }

        self.shared.get_clipboard()
    }

    #[inline]
    fn set_clipboard<'c>(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self
            .window
            .as_ref()
            .and_then(|data| data.wayland_clipboard.as_ref())
        {
            cb.store(content);
            return;
        }

        self.shared.set_clipboard(content);
    }

    #[inline]
    fn get_primary(&mut self) -> Option<String> {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self
            .window
            .as_ref()
            .and_then(|data| data.wayland_clipboard.as_ref())
        {
            return match cb.load_primary() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn_about_error("Failed to get clipboard contents", &e);
                    None
                }
            };
        }

        self.shared.get_primary()
    }

    #[inline]
    fn set_primary<'c>(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self
            .window
            .as_ref()
            .and_then(|data| data.wayland_clipboard.as_ref())
        {
            cb.store_primary(content);
            return;
        }

        self.shared.set_primary(content);
    }

    fn adjust_theme<'s>(&'s mut self, f: Box<dyn FnOnce(&mut dyn ThemeControl) -> Action + 's>) {
        let action = f(&mut self.shared.theme);
        self.shared.pending.push(PendingAction::Action(action));
    }

    fn size_and_draw_shared<'s>(
        &'s mut self,
        f: Box<dyn FnOnce(&mut dyn ThemeSize, &mut dyn DrawShared) + 's>,
    ) {
        use kas::theme::Window;
        let mut size = self.theme_window.size();
        f(&mut size, &mut self.shared.draw);
    }

    #[inline]
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        if let Some(window) = self.window {
            window.set_cursor_icon(icon);
        }
    }

    fn platform(&self) -> Platform {
        self.shared.platform
    }

    #[cfg(features = "winit")]
    #[inline]
    fn winit_window(&self) -> Option<&winit::window::Window> {
        self.window
    }

    #[inline]
    fn waker(&self) -> &std::task::Waker {
        &self.shared.waker
    }
}
