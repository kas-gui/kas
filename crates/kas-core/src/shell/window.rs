// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window types

use super::common::WindowSurface;
use super::shared::{SharedState, ShellShared};
use super::ProxyAction;
use kas::cast::Cast;
use kas::draw::{color::Rgba, AnimationState};
use kas::event::{config::WindowConfig, ConfigCx, CursorIcon, EventState};
use kas::geom::{Coord, Rect, Size};
use kas::layout::SolveCache;
use kas::theme::{DrawCx, SizeCx, ThemeSize};
use kas::theme::{Theme, Window as _};
use kas::{autoimpl, Action, AppData, ErasedStack, Id, Layout, LayoutExt, Widget, WindowId};
use std::mem::take;
use std::time::{Duration, Instant};
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

/// Window fields requiring a frame or surface
#[crate::autoimpl(Deref, DerefMut using self.window)]
struct WindowData<S: WindowSurface, T: Theme<S::Shared>> {
    window: winit::window::Window,
    #[cfg(all(wayland_platform, feature = "clipboard"))]
    wayland_clipboard: Option<smithay_clipboard::Clipboard>,
    surface: S,
    /// Frame rate counter
    frame_count: (Instant, u32),

    // NOTE: cached components could be here or in Window
    window_id: WindowId,
    solve_cache: SolveCache,
    theme_window: T::Window,
    next_avail_frame_time: Instant,
    queued_frame_time: Option<Instant>,
}

/// Per-window data
#[autoimpl(Debug ignore self._data, self.widget, self.ev_state, self.window)]
pub struct Window<A: AppData, S: WindowSurface, T: Theme<S::Shared>> {
    _data: std::marker::PhantomData<A>,
    pub(super) widget: kas::Window<A>,
    pub(super) window_id: WindowId,
    ev_state: EventState,
    window: Option<WindowData<S, T>>,
}

// Public functions, for use by the toolkit
impl<A: AppData, S: WindowSurface, T: Theme<S::Shared>> Window<A, S, T> {
    /// Construct window state (widget)
    pub(super) fn new(
        shared: &ShellShared<A, S, T>,
        window_id: WindowId,
        widget: kas::Window<A>,
    ) -> Self {
        let config = WindowConfig::new(shared.config.clone());
        Window {
            _data: std::marker::PhantomData,
            widget,
            window_id,
            ev_state: EventState::new(config, shared.platform),
            window: None,
        }
    }

    /// Open (resume) a window
    pub(super) fn resume(
        &mut self,
        shared: &mut SharedState<A, S, T>,
        elwt: &EventLoopWindowTarget<ProxyAction>,
    ) -> super::Result<winit::window::WindowId> {
        let time = Instant::now();

        // Wayland only supports windows constructed via logical size
        let use_logical_size = shared.shell.platform.is_wayland();

        let scale_factor = if use_logical_size {
            1.0
        } else {
            shared.scale_factor as f32
        };

        let mut theme_window = shared.shell.theme.new_window(scale_factor);
        let dpem = theme_window.size().dpem();

        self.ev_state.update_config(scale_factor, dpem);
        self.ev_state.full_configure(
            theme_window.size(),
            self.window_id,
            &mut self.widget,
            &shared.data,
        );

        let node = self.widget.as_node(&shared.data);
        let sizer = SizeCx::new(theme_window.size());
        let mut solve_cache = SolveCache::find_constraints(node, sizer);

        // Opening a zero-size window causes a crash, so force at least 1x1:
        let ideal = solve_cache.ideal(true).max(Size(1, 1));
        let ideal = match use_logical_size {
            false => ideal.as_physical(),
            true => ideal.as_logical(),
        };

        let mut builder = WindowBuilder::new().with_inner_size(ideal);
        let (restrict_min, restrict_max) = self.widget.restrictions();
        if restrict_min {
            let min = solve_cache.min(true);
            let min = match use_logical_size {
                false => min.as_physical(),
                true => min.as_logical(),
            };
            builder = builder.with_min_inner_size(min);
        }
        if restrict_max {
            builder = builder.with_max_inner_size(ideal);
        }
        let window = builder
            .with_title(self.widget.title())
            .with_window_icon(self.widget.icon())
            .with_decorations(self.widget.decorations() == kas::Decorations::Server)
            .with_transparent(self.widget.transparent())
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
            shared
                .shell
                .theme
                .update_window(&mut theme_window, scale_factor);
            let dpem = theme_window.size().dpem();
            self.ev_state.update_config(scale_factor, dpem);
            solve_cache.invalidate_rule_cache();
        }

        #[cfg(all(wayland_platform, feature = "clipboard"))]
        use raw_window_handle::{HasRawDisplayHandle, RawDisplayHandle, WaylandDisplayHandle};
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        let wayland_clipboard = match window.raw_display_handle() {
            RawDisplayHandle::Wayland(WaylandDisplayHandle { display, .. }) => {
                Some(unsafe { smithay_clipboard::Clipboard::new(display) })
            }
            _ => None,
        };

        let surface = S::new(&mut shared.shell.draw.draw, size, &window)?;

        let winit_id = window.id();

        self.window = Some(WindowData {
            window,
            #[cfg(all(wayland_platform, feature = "clipboard"))]
            wayland_clipboard,
            surface,
            frame_count: (Instant::now(), 0),

            window_id: self.window_id,
            solve_cache,
            theme_window,
            next_avail_frame_time: time,
            queued_frame_time: Some(time),
        });

        self.apply_size(shared, true);

        log::trace!(target: "kas_perf::wgpu::window", "resume: {}µs", time.elapsed().as_micros());
        Ok(winit_id)
    }

    /// Close (suspend) the window, keeping state (widget)
    pub(super) fn suspend(&mut self) {
        // TODO: close popups and notify the widget to allow saving data
        self.window = None;
    }

    /// Handle an event
    ///
    /// Returns `true` to force polling temporarily.
    pub(super) fn handle_event(
        &mut self,
        shared: &mut SharedState<A, S, T>,
        event: WindowEvent,
    ) -> bool {
        let Some(ref mut window) = self.window else {
            return false;
        };
        match event {
            WindowEvent::Moved(_) | WindowEvent::Destroyed => false,
            WindowEvent::Resized(size) => {
                if window
                    .surface
                    .do_resize(&mut shared.shell.draw.draw, size.cast())
                {
                    self.apply_size(shared, false);
                }
                false
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Note: API allows us to set new window size here.
                shared.scale_factor = scale_factor;
                let scale_factor = scale_factor as f32;
                shared
                    .shell
                    .theme
                    .update_window(&mut window.theme_window, scale_factor);
                let dpem = window.theme_window.size().dpem();
                self.ev_state.update_config(scale_factor, dpem);
                window.solve_cache.invalidate_rule_cache();
                false
            }
            WindowEvent::RedrawRequested => self.do_draw(shared).is_err(),
            event => {
                let mut messages = ErasedStack::new();
                self.ev_state
                    .with(&mut shared.shell, window, &mut messages, |cx| {
                        cx.handle_winit(&mut self.widget, &shared.data, event);
                    });
                shared.handle_messages(&mut messages);

                if self.ev_state.action.contains(Action::RECONFIGURE) {
                    // Reconfigure must happen before further event handling
                    self.reconfigure(shared);
                    self.ev_state.action.remove(Action::RECONFIGURE);
                }
                false
            }
        }
    }

    /// Handle all pending items before event loop sleeps
    pub(super) fn flush_pending(
        &mut self,
        shared: &mut SharedState<A, S, T>,
    ) -> (Action, Option<Instant>) {
        let Some(ref window) = self.window else {
            return (Action::empty(), None);
        };
        let mut messages = ErasedStack::new();
        let action = self.ev_state.flush_pending(
            &mut shared.shell,
            window,
            &mut messages,
            &mut self.widget,
            &shared.data,
        );
        shared.handle_messages(&mut messages);

        if action.contains(Action::CLOSE | Action::EXIT) {
            return (action, None);
        }
        self.handle_action(shared, action);

        let mut resume = self.ev_state.next_resume();

        let window = self.window.as_mut().unwrap();
        if let Some(time) = window.queued_frame_time {
            if time <= Instant::now() {
                window.request_redraw();
                window.queued_frame_time = None;
            } else {
                resume = resume.map(|t| t.min(time)).or(Some(time));
            }
        }

        (action, resume)
    }

    /// Handle an action (excludes handling of CLOSE and EXIT)
    pub(super) fn handle_action(&mut self, shared: &SharedState<A, S, T>, mut action: Action) {
        if action.contains(Action::EVENT_CONFIG) {
            if let Some(ref mut window) = self.window {
                let scale_factor = window.scale_factor() as f32;
                let dpem = window.theme_window.size().dpem();
                self.ev_state.update_config(scale_factor, dpem);
                action |= Action::UPDATE;
            }
        }
        if action.contains(Action::RECONFIGURE) {
            self.reconfigure(shared);
        } else if action.contains(Action::UPDATE) {
            self.update(shared);
        }
        if action.contains(Action::THEME_UPDATE) {
            if let Some(ref mut window) = self.window {
                let scale_factor = window.scale_factor() as f32;
                shared
                    .shell
                    .theme
                    .update_window(&mut window.theme_window, scale_factor);
            }
        }
        if action.contains(Action::RESIZE) {
            if let Some(ref mut window) = self.window {
                window.solve_cache.invalidate_rule_cache();
            }
            self.apply_size(shared, false);
        } else if !(action & (Action::SET_RECT | Action::SCROLLED)).is_empty() {
            self.apply_size(shared, false);
        }
        debug_assert!(!action.contains(Action::REGION_MOVED));
        if !action.is_empty() {
            if let Some(ref mut window) = self.window {
                window.queued_frame_time = Some(window.next_avail_frame_time);
            }
        }
    }

    pub(super) fn update_timer(&mut self, shared: &mut SharedState<A, S, T>) -> Option<Instant> {
        let Some(ref window) = self.window else {
            return None;
        };
        let widget = self.widget.as_node(&shared.data);
        let mut messages = ErasedStack::new();
        self.ev_state
            .with(&mut shared.shell, window, &mut messages, |cx| {
                cx.update_timer(widget)
            });
        shared.handle_messages(&mut messages);
        self.next_resume()
    }

    pub(super) fn add_popup(
        &mut self,
        shared: &mut SharedState<A, S, T>,
        id: WindowId,
        popup: kas::PopupDescriptor,
    ) {
        let Some(ref window) = self.window else {
            return;
        };
        let mut messages = ErasedStack::new();
        self.ev_state
            .with(&mut shared.shell, window, &mut messages, |cx| {
                self.widget.add_popup(cx, &shared.data, id, popup)
            });
        shared.handle_messages(&mut messages);
    }

    pub(super) fn send_action(&mut self, action: Action) {
        self.ev_state.action(Id::ROOT, action);
    }

    pub(super) fn send_close(&mut self, shared: &mut SharedState<A, S, T>, id: WindowId) {
        if id == self.window_id {
            self.ev_state.action(Id::ROOT, Action::CLOSE);
        } else if let Some(window) = self.window.as_ref() {
            let widget = &mut self.widget;
            let mut messages = ErasedStack::new();
            self.ev_state
                .with(&mut shared.shell, window, &mut messages, |cx| {
                    widget.remove_popup(cx, id)
                });
            shared.handle_messages(&mut messages);
        }
    }
}

// Internal functions
impl<A: AppData, S: WindowSurface, T: Theme<S::Shared>> Window<A, S, T> {
    fn reconfigure(&mut self, shared: &SharedState<A, S, T>) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        self.ev_state.full_configure(
            window.theme_window.size(),
            self.window_id,
            &mut self.widget,
            &shared.data,
        );

        log::trace!(target: "kas_perf::wgpu::window", "reconfigure: {}µs", time.elapsed().as_micros());
    }

    fn update(&mut self, shared: &SharedState<A, S, T>) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        let size = window.theme_window.size();
        let mut cx = ConfigCx::new(&size, &mut self.ev_state);
        cx.update(self.widget.as_node(&shared.data));

        log::trace!(target: "kas_perf::wgpu::window", "update: {}µs", time.elapsed().as_micros());
    }

    fn apply_size(&mut self, shared: &SharedState<A, S, T>, first: bool) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };
        let rect = Rect::new(Coord::ZERO, window.surface.size());
        log::debug!("apply_size: rect={rect:?}");

        let solve_cache = &mut window.solve_cache;
        let mut cx = ConfigCx::new(window.theme_window.size(), &mut self.ev_state);
        solve_cache.apply_rect(self.widget.as_node(&shared.data), &mut cx, rect, true);
        if first {
            solve_cache.print_widget_heirarchy(self.widget.as_layout());
        }
        self.widget.resize_popups(&mut cx, &shared.data);

        let (restrict_min, restrict_max) = self.widget.restrictions();
        if restrict_min {
            let min = window.solve_cache.min(true).as_physical();
            window.set_min_inner_size(Some(min));
        };
        if restrict_max {
            let ideal = window.solve_cache.ideal(true).as_physical();
            window.set_max_inner_size(Some(ideal));
        };

        window.request_redraw();
        log::trace!(
            target: "kas_perf::wgpu::window",
            "apply_size: {}µs", time.elapsed().as_micros(),
        );
    }

    /// Draw
    ///
    /// Returns an error when drawing is aborted and further event handling may
    /// be needed before a redraw.
    pub(super) fn do_draw(&mut self, shared: &mut SharedState<A, S, T>) -> Result<(), ()> {
        let start = Instant::now();
        let Some(ref mut window) = self.window else {
            return Ok(());
        };

        window.next_avail_frame_time = start + self.ev_state.config().frame_dur();

        {
            let draw = window.surface.draw_iface(&mut shared.shell.draw);

            let mut draw =
                shared
                    .shell
                    .theme
                    .draw(draw, &mut self.ev_state, &mut window.theme_window);
            let draw_cx = DrawCx::new(&mut draw, self.widget.id());
            self.widget.draw(&shared.data, draw_cx);
        }
        let time2 = Instant::now();

        let anim = take(&mut window.surface.common_mut().anim);
        window.queued_frame_time = match anim {
            AnimationState::None => None,
            AnimationState::Animate => Some(window.next_avail_frame_time),
            AnimationState::Timed(time) => Some(time.max(window.next_avail_frame_time)),
        };
        self.ev_state.action -= Action::REDRAW; // we just drew
        if !self.ev_state.action.is_empty() {
            log::info!("do_draw: abort and enqueue `Self::update` due to non-empty actions");
            return Err(());
        }

        let clear_color = if self.widget.transparent() {
            Rgba::TRANSPARENT
        } else {
            shared.shell.theme.clear_color()
        };
        let time3 = window
            .surface
            .present(&mut shared.shell.draw.draw, clear_color);

        let text_dur_micros = take(&mut window.surface.common_mut().dur_text);
        let end = Instant::now();
        log::trace!(
            target: "kas_perf::wgpu::window",
            "do_draw: {}μs ({}μs widgets, {}μs text, {}μs render, {}μs present)",
            (end - start).as_micros(),
            (time2 - start).as_micros(),
            text_dur_micros.as_micros(),
            (time3 - time2).as_micros(),
            (end - time2).as_micros()
        );

        const SECOND: Duration = Duration::from_secs(1);
        window.frame_count.1 += 1;
        let now = Instant::now();
        if window.frame_count.0 + SECOND <= now {
            log::debug!(
                "Window {:?}: {} frames in last second",
                window.window_id,
                window.frame_count.1
            );
            window.frame_count.0 = now;
            window.frame_count.1 = 0;
        }

        Ok(())
    }

    fn next_resume(&self) -> Option<Instant> {
        self.window.as_ref().and_then(|w| {
            match (self.ev_state.next_resume(), w.queued_frame_time) {
                (Some(t1), Some(t2)) => Some(t1.min(t2)),
                (Some(t), None) => Some(t),
                (None, Some(t)) => Some(t),
                (None, None) => None,
            }
        })
    }
}

pub(crate) trait WindowDataErased {
    /// Get the window identifier
    fn window_id(&self) -> WindowId;

    /// Access the wayland clipboard object, if available
    #[cfg(all(wayland_platform, feature = "clipboard"))]
    fn wayland_clipboard(&self) -> Option<&smithay_clipboard::Clipboard>;

    /// Access the [`ThemeSize`] object
    fn theme_size(&self) -> &dyn ThemeSize;

    /// Set the mouse cursor
    fn set_cursor_icon(&self, icon: CursorIcon);

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    #[cfg(winit)]
    fn winit_window(&self) -> Option<&winit::window::Window>;
}

impl<S: WindowSurface, T: Theme<S::Shared>> WindowDataErased for WindowData<S, T> {
    fn window_id(&self) -> WindowId {
        self.window_id
    }

    #[cfg(all(wayland_platform, feature = "clipboard"))]
    fn wayland_clipboard(&self) -> Option<&smithay_clipboard::Clipboard> {
        self.wayland_clipboard.as_ref()
    }

    fn theme_size(&self) -> &dyn ThemeSize {
        self.theme_window.size()
    }

    #[inline]
    fn set_cursor_icon(&self, icon: CursorIcon) {
        self.window.set_cursor_icon(icon);
    }

    #[cfg(winit)]
    #[inline]
    fn winit_window(&self) -> Option<&winit::window::Window> {
        Some(&self.window)
    }
}
