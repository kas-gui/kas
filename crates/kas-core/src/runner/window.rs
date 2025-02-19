// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window types

use super::common::WindowSurface;
use super::shared::{SharedState, State};
use super::{AppData, GraphicsBuilder};
use crate::cast::{Cast, Conv};
use crate::config::WindowConfig;
use crate::decorations::Decorations;
use crate::draw::{color::Rgba, AnimationState, DrawSharedImpl};
use crate::event::{ConfigCx, CursorIcon, EventState};
use crate::geom::{Coord, Rect, Size};
use crate::layout::SolveCache;
use crate::messages::MessageStack;
use crate::theme::{DrawCx, SizeCx, Theme, ThemeSize, Window as _};
use crate::{autoimpl, Action, Id, Tile, Widget, WindowId};
use std::mem::take;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;

/// Window fields requiring a frame or surface
#[crate::autoimpl(Deref, DerefMut using self.window)]
struct WindowData<G: GraphicsBuilder, T: Theme<G::Shared>> {
    window: Arc<winit::window::Window>,
    #[cfg(all(wayland_platform, feature = "clipboard"))]
    wayland_clipboard: Option<smithay_clipboard::Clipboard>,
    surface: G::Surface<'static>,
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
pub struct Window<A: AppData, G: GraphicsBuilder, T: Theme<G::Shared>> {
    _data: std::marker::PhantomData<A>,
    pub(super) widget: kas::Window<A>,
    ev_state: EventState,
    window: Option<WindowData<G, T>>,
}

// Public functions, for use by the toolkit
impl<A: AppData, G: GraphicsBuilder, T: Theme<G::Shared>> Window<A, G, T> {
    /// Construct window state (widget)
    pub(super) fn new(
        shared: &SharedState<A, G, T>,
        window_id: WindowId,
        widget: kas::Window<A>,
    ) -> Self {
        let config = WindowConfig::new(shared.config.clone());
        Window {
            _data: std::marker::PhantomData,
            widget,
            ev_state: EventState::new(window_id, config, shared.platform),
            window: None,
        }
    }

    #[inline]
    pub(super) fn window_id(&self) -> WindowId {
        self.ev_state.window_id
    }

    /// Open (resume) a window
    pub(super) fn resume(
        &mut self,
        state: &mut State<A, G, T>,
        el: &ActiveEventLoop,
    ) -> super::Result<winit::window::WindowId> {
        let time = Instant::now();

        // We cannot reliably determine the scale factor before window creation.
        // A factor of 1.0 lets us estimate the size requirements (logical).
        self.ev_state.update_config(1.0);

        let config = self.ev_state.config();
        let mut theme_window = state.shared.theme.new_window(config);

        self.ev_state
            .full_configure(theme_window.size(), &mut self.widget, &state.data);

        let node = self.widget.as_node(&state.data);
        let sizer = SizeCx::new(theme_window.size());
        let mut solve_cache = SolveCache::find_constraints(node, sizer);

        // Opening a zero-size window causes a crash, so force at least 1x1:
        let min_size = Size(1, 1);
        let max_size = Size::splat(state.shared.draw.draw.max_texture_dimension_2d().cast());

        let ideal = solve_cache
            .ideal(true)
            .clamp(min_size, max_size)
            .as_logical();

        let mut attrs = WindowAttributes::default();
        attrs.inner_size = Some(ideal);
        attrs.title = self.widget.title().to_string();
        attrs.visible = false;
        attrs.transparent = self.widget.transparent();
        attrs.decorations = self.widget.decorations() == Decorations::Server;
        attrs.window_icon = self.widget.icon();
        let (restrict_min, restrict_max) = self.widget.restrictions();
        if restrict_min {
            let min = solve_cache.min(true).as_logical();
            attrs.min_inner_size = Some(min);
        }
        if restrict_max {
            attrs.max_inner_size = Some(ideal);
        }
        let window = el.create_window(attrs)?;

        // Now that we have a scale factor, we may need to resize:
        let scale_factor = window.scale_factor();
        if scale_factor != 1.0 {
            self.ev_state.update_config(scale_factor as f32);

            let config = self.ev_state.config();
            state.shared.theme.update_window(&mut theme_window, config);

            let node = self.widget.as_node(&state.data);
            let sizer = SizeCx::new(theme_window.size());
            solve_cache = SolveCache::find_constraints(node, sizer);

            // NOTE: we would use .as_physical(), but we need to ensure rounding
            // doesn't result in anything exceeding max_size which can happen
            // otherwise (default rounding mode is to nearest, away from zero).
            let ideal = solve_cache.ideal(true).max(min_size);
            let ub = (f64::conv(max_size.0) / scale_factor).floor();
            let w = ub.min(f64::conv(ideal.0) / scale_factor);
            let h = ub.min(f64::conv(ideal.1) / scale_factor);
            let ideal = winit::dpi::LogicalSize::new(w, h);

            if let Some(size) = window.request_inner_size(ideal) {
                debug_assert_eq!(size, window.inner_size());
            } else {
                // We will receive WindowEvent::Resized and resize then.
                // Unfortunately we can't rely on this since some platforms (X11)
                // don't always behave as expected, thus we must resize now.
            }
        }

        let size: Size = window.inner_size().cast();
        log::info!(
            "Window::resume: constructed with physical size {:?}, scale factor {}",
            size,
            scale_factor,
        );

        #[cfg(all(wayland_platform, feature = "clipboard"))]
        use raw_window_handle::{HasDisplayHandle, RawDisplayHandle, WaylandDisplayHandle};
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        let wayland_clipboard = match window.display_handle() {
            Ok(handle) => match handle.as_raw() {
                RawDisplayHandle::Wayland(WaylandDisplayHandle { display, .. }) => {
                    Some(unsafe { smithay_clipboard::Clipboard::new(display.as_ptr()) })
                }
                _ => None,
            },
            _ => None,
        };

        // NOTE: usage of Arc is inelegant, but avoids lots of unsafe code
        let window = Arc::new(window);
        let mut surface = G::new_surface(
            &mut state.shared.draw.draw,
            window.clone(),
            self.widget.transparent(),
        )?;
        surface.do_resize(&mut state.shared.draw.draw, size);

        let winit_id = window.id();

        self.window = Some(WindowData {
            window,
            #[cfg(all(wayland_platform, feature = "clipboard"))]
            wayland_clipboard,
            surface,
            frame_count: (Instant::now(), 0),

            window_id: self.ev_state.window_id,
            solve_cache,
            theme_window,
            next_avail_frame_time: time,
            queued_frame_time: Some(time),
        });

        self.apply_size(state, true);

        log::trace!(target: "kas_perf::wgpu::window", "resume: {}µs", time.elapsed().as_micros());
        Ok(winit_id)
    }

    /// Close (suspend) the window, keeping state (widget)
    ///
    /// Returns `true` unless this `Window` should be destoyed.
    pub(super) fn suspend(&mut self, state: &mut State<A, G, T>) -> bool {
        if let Some(ref mut window) = self.window {
            self.ev_state.suspended(&mut state.shared);

            let mut messages = MessageStack::new();
            let action = self.ev_state.flush_pending(
                &mut state.shared,
                window,
                &mut messages,
                &mut self.widget,
                &state.data,
            );
            state.handle_messages(&mut messages);

            self.window = None;
            !action.contains(Action::CLOSE)
        } else {
            true
        }
    }

    /// Handle an event
    ///
    /// Returns `true` to force polling temporarily.
    pub(super) fn handle_event(&mut self, state: &mut State<A, G, T>, event: WindowEvent) -> bool {
        let Some(ref mut window) = self.window else {
            return false;
        };

        match event {
            WindowEvent::Moved(_) | WindowEvent::Destroyed => false,
            WindowEvent::Resized(size) => {
                if window
                    .surface
                    .do_resize(&mut state.shared.draw.draw, size.cast())
                {
                    self.apply_size(state, false);
                }
                false
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Note: API allows us to set new window size here.
                self.ev_state.update_config(scale_factor as f32);

                let config = self.ev_state.config();
                state
                    .shared
                    .theme
                    .update_window(&mut window.theme_window, config);

                // NOTE: we could try resizing here in case the window is too
                // small due to non-linear scaling, but it appears unnecessary.
                window.solve_cache.invalidate_rule_cache();
                false
            }
            WindowEvent::RedrawRequested => self.do_draw(state).is_err(),
            event => {
                let mut messages = MessageStack::new();
                self.ev_state
                    .with(&mut state.shared, window, &mut messages, |cx| {
                        cx.handle_winit(&mut self.widget, &state.data, event);
                    });
                state.handle_messages(&mut messages);

                if self.ev_state.action.contains(Action::RECONFIGURE) {
                    // Reconfigure must happen before further event handling
                    self.reconfigure(state);
                    self.ev_state.action.remove(Action::RECONFIGURE);
                }
                false
            }
        }
    }

    /// Handle all pending items before event loop sleeps
    pub(super) fn flush_pending(
        &mut self,
        state: &mut State<A, G, T>,
    ) -> (Action, Option<Instant>) {
        let Some(ref window) = self.window else {
            return (Action::empty(), None);
        };

        let mut messages = MessageStack::new();
        let action = self.ev_state.flush_pending(
            &mut state.shared,
            window,
            &mut messages,
            &mut self.widget,
            &state.data,
        );
        state.handle_messages(&mut messages);

        if action.contains(Action::CLOSE) {
            return (action, None);
        }
        self.handle_action(state, action);

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
    pub(super) fn handle_action(&mut self, state: &mut State<A, G, T>, mut action: Action) {
        if action.contains(Action::EVENT_CONFIG) {
            if let Some(ref mut window) = self.window {
                self.ev_state.update_config(window.scale_factor() as f32);
                action |= Action::UPDATE;
            }
        }
        if action.contains(Action::RECONFIGURE) {
            self.reconfigure(state);
        } else if action.contains(Action::UPDATE) {
            self.update(state);
        }
        if action.contains(Action::THEME_SWITCH) {
            if let Some(ref mut window) = self.window {
                let config = self.ev_state.config();
                window.theme_window = state.shared.theme.new_window(config);
            }
            action |= Action::RESIZE;
        } else if action.contains(Action::THEME_UPDATE) {
            if let Some(ref mut window) = self.window {
                let config = self.ev_state.config();
                state
                    .shared
                    .theme
                    .update_window(&mut window.theme_window, config);
            }
            action |= Action::RESIZE;
        }
        if action.contains(Action::RESIZE) {
            if let Some(ref mut window) = self.window {
                window.solve_cache.invalidate_rule_cache();
            }
            self.apply_size(state, false);
        } else if !(action & (Action::SET_RECT | Action::SCROLLED)).is_empty() {
            self.apply_size(state, false);
        }
        debug_assert!(!action.contains(Action::REGION_MOVED));
        if !action.is_empty() {
            if let Some(ref mut window) = self.window {
                window.queued_frame_time = Some(window.next_avail_frame_time);
            }
        }
    }

    pub(super) fn update_timer(&mut self, state: &mut State<A, G, T>) -> Option<Instant> {
        let window = self.window.as_ref()?;

        let widget = self.widget.as_node(&state.data);
        let mut messages = MessageStack::new();
        self.ev_state
            .with(&mut state.shared, window, &mut messages, |cx| {
                cx.update_timer(widget)
            });
        state.handle_messages(&mut messages);
        self.next_resume()
    }

    pub(super) fn add_popup(
        &mut self,
        state: &mut State<A, G, T>,
        id: WindowId,
        popup: kas::PopupDescriptor,
    ) {
        let Some(ref window) = self.window else {
            return;
        };

        let size = window.theme_window.size();
        let mut cx = ConfigCx::new(&size, &mut self.ev_state);
        self.widget.add_popup(&mut cx, &state.data, id, popup);
    }

    pub(super) fn send_action(&mut self, action: Action) {
        self.ev_state.action(Id::ROOT, action);
    }

    pub(super) fn send_close(&mut self, id: WindowId) {
        if id == self.ev_state.window_id {
            self.ev_state.action(Id::ROOT, Action::CLOSE);
        } else if let Some(window) = self.window.as_ref() {
            let widget = &mut self.widget;
            let size = window.theme_window.size();
            let mut cx = ConfigCx::new(&size, &mut self.ev_state);
            widget.remove_popup(&mut cx, id);
        }
    }
}

// Internal functions
impl<A: AppData, G: GraphicsBuilder, T: Theme<G::Shared>> Window<A, G, T> {
    fn reconfigure(&mut self, state: &State<A, G, T>) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        self.ev_state
            .full_configure(window.theme_window.size(), &mut self.widget, &state.data);

        log::trace!(target: "kas_perf::wgpu::window", "reconfigure: {}µs", time.elapsed().as_micros());
    }

    fn update(&mut self, state: &State<A, G, T>) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        let size = window.theme_window.size();
        let mut cx = ConfigCx::new(&size, &mut self.ev_state);
        cx.update(self.widget.as_node(&state.data));

        log::trace!(target: "kas_perf::wgpu::window", "update: {}µs", time.elapsed().as_micros());
    }

    fn apply_size(&mut self, state: &State<A, G, T>, first: bool) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };
        let rect = Rect::new(Coord::ZERO, window.surface.size());
        log::debug!("apply_size: rect={rect:?}");

        let solve_cache = &mut window.solve_cache;
        let mut cx = ConfigCx::new(window.theme_window.size(), &mut self.ev_state);
        solve_cache.apply_rect(self.widget.as_node(&state.data), &mut cx, rect, true);
        if first {
            solve_cache.print_widget_heirarchy(self.widget.as_tile());
        }
        self.widget.resize_popups(&mut cx, &state.data);

        // Size restrictions may have changed due to content or size (line wrapping)
        let (restrict_min, restrict_max) = self.widget.restrictions();
        if restrict_min {
            let min = window.solve_cache.min(true).as_physical();
            window.set_min_inner_size(Some(min));
        };
        if restrict_max {
            let ideal = window.solve_cache.ideal(true).as_physical();
            window.set_max_inner_size(Some(ideal));
        };

        window.set_visible(true);
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
    pub(super) fn do_draw(&mut self, state: &mut State<A, G, T>) -> Result<(), ()> {
        let start = Instant::now();
        let Some(ref mut window) = self.window else {
            return Ok(());
        };

        window.next_avail_frame_time = start + self.ev_state.config().frame_dur();

        {
            let draw = window.surface.draw_iface(&mut state.shared.draw);

            let mut draw =
                state
                    .shared
                    .theme
                    .draw(draw, &mut self.ev_state, &mut window.theme_window);
            let draw_cx = DrawCx::new(&mut draw, self.widget.id());
            self.widget.draw(draw_cx);
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
            state.shared.theme.clear_color()
        };
        let time3 = window
            .surface
            .present(&mut state.shared.draw.draw, clear_color);

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

impl<G: GraphicsBuilder, T: Theme<G::Shared>> WindowDataErased for WindowData<G, T> {
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
        self.window.set_cursor(icon);
    }

    #[cfg(winit)]
    #[inline]
    fn winit_window(&self) -> Option<&winit::window::Window> {
        Some(&self.window)
    }
}
