// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window types

use super::common::WindowSurface;
use super::shared::Shared;
use super::{AppData, GraphicsInstance, Platform};
use crate::cast::{Cast, CastApprox};
use crate::config::{Config, WindowConfig};
use crate::draw::PassType;
use crate::draw::color::Rgba;
use crate::event::{ConfigCx, CursorIcon, EventState};
use crate::geom::{Coord, Offset, Rect, Size};
use crate::layout::SolveCache;
use crate::messages::Erased;
use crate::theme::{DrawCx, SizeCx, Theme, ThemeDraw, ThemeSize, Window as _};
use crate::window::{BoxedWindow, Decorations, PopupDescriptor, WindowId, WindowWidget};
use crate::{Action, Id, Layout, Tile, Widget, autoimpl};
#[cfg(windows_platform)]
use raw_window_handle::HasWindowHandle;
use std::cell::RefCell;
use std::mem::take;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{ImePurpose, WindowAttributes};

/// Window fields requiring a frame or surface
#[crate::autoimpl(Deref, DerefMut using self.window)]
struct WindowData<G: GraphicsInstance, T: Theme<G::Shared>> {
    window: Arc<winit::window::Window>,
    #[cfg(all(wayland_platform, feature = "clipboard"))]
    wayland_clipboard: Option<smithay_clipboard::Clipboard>,
    surface: G::Surface<'static>,
    /// Frame rate counter
    frame_count: (Instant, u32),
    #[cfg(feature = "accesskit")]
    accesskit: accesskit_winit::Adapter,

    // NOTE: cached components could be here or in Window
    window_id: WindowId,
    solve_cache: SolveCache,
    theme_window: T::Window,
    need_redraw: bool,
}

/// Per-window data
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
#[autoimpl(Debug ignore self._data, self.widget, self.ev_state, self.window)]
pub struct Window<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> {
    _data: std::marker::PhantomData<A>,
    pub(super) widget: Box<dyn WindowWidget<Data = A>>,
    ev_state: EventState,
    window: Option<WindowData<G, T>>,
}

// Public functions, for use by the toolkit
impl<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> Window<A, G, T> {
    /// Construct window state (widget)
    pub fn new(
        config: Rc<RefCell<Config>>,
        platform: Platform,
        window_id: WindowId,
        widget: BoxedWindow<A>,
    ) -> Self {
        let config = WindowConfig::new(config);
        Window {
            _data: std::marker::PhantomData,
            widget: widget.0,
            ev_state: EventState::new(window_id, config, platform),
            window: None,
        }
    }

    #[inline]
    pub(super) fn window_id(&self) -> WindowId {
        self.ev_state.window_id
    }

    #[inline]
    pub(super) fn winit_window(&self) -> Option<&winit::window::Window> {
        self.window.as_ref().map(|d| &*d.window)
    }

    /// Open (resume) a window
    pub(super) fn resume(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
        el: &ActiveEventLoop,
        #[allow(unused)] modal_parent: Option<&winit::window::Window>,
    ) -> super::Result<winit::window::WindowId> {
        let time = Instant::now();

        // We use the logical size and scale factor of the largest monitor as
        // an upper bound on window size and guessed scale factor.
        let mut max_physical_size = PhysicalSize::new(800, 600);
        let mut scale_factor = 1.0;
        let mut product = 0;
        for monitor in el.available_monitors() {
            let size = monitor.size();
            let p = size.width * size.height;
            if p > product {
                product = p;
                max_physical_size = size;
                scale_factor = monitor.scale_factor();
            }
        }
        let max_size = max_physical_size.to_logical::<f64>(scale_factor);

        self.ev_state.update_config(scale_factor.cast_approx());
        let config = self.ev_state.config();
        let mut theme_window = shared.theme.new_window(config);

        let mut node = self.widget.as_node(data);
        self.ev_state.full_configure(theme_window.size(), node.re());

        let sizer = SizeCx::new(theme_window.size());
        let mut solve_cache = SolveCache::find_constraints(node, sizer);

        // Opening a zero-size window causes a crash, so force at least 1x1:
        let min_size = Size(1, 1);
        let mut ideal = solve_cache
            .ideal(true)
            .max(min_size)
            .as_physical()
            .to_logical(scale_factor);
        if ideal.width > max_size.width {
            ideal.width = max_size.width;
        }
        if ideal.height > max_size.height {
            ideal.height = max_size.height;
        }

        let props = self.widget.properties();
        let mut attrs = WindowAttributes::default();
        attrs.inner_size = Some(ideal.into());
        attrs.title = self.widget.title().to_string();
        attrs.visible = false;
        let transparent = props.transparent();
        attrs.transparent = transparent;
        attrs.decorations = props.decorations() == Decorations::Server;
        attrs.window_icon = props.icon();
        let (restrict_min, restrict_max) = props.restrictions();
        if restrict_min {
            let min = solve_cache
                .min(true)
                .as_physical()
                .to_logical::<f64>(scale_factor);
            attrs.min_inner_size = Some(min.into());
        }
        if restrict_max {
            attrs.max_inner_size = Some(ideal.into());
        } else {
            attrs.max_inner_size = Some(max_size.into());
        }
        #[cfg(windows_platform)]
        if let Some(handle) = modal_parent.and_then(|p| p.window_handle().ok()) {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(true);
            match handle.as_raw() {
                raw_window_handle::RawWindowHandle::Win32(h) => {
                    attrs = attrs.with_owner_window(h.hwnd.get());
                }
                _ => (),
            }
        }
        let window = el.create_window(attrs)?;

        // Now that we have a scale factor, we may need to resize:
        let new_factor = window.scale_factor();
        if new_factor != scale_factor {
            scale_factor = new_factor;
            self.ev_state.update_config(scale_factor as f32);

            let config = self.ev_state.config();
            shared.theme.update_window(&mut theme_window, config);

            // Update text size which is assigned during configure
            let mut node = self.widget.as_node(data);
            self.ev_state.full_configure(theme_window.size(), node.re());

            let sizer = SizeCx::new(theme_window.size());
            solve_cache = SolveCache::find_constraints(node, sizer);

            if let Some(monitor) = window.current_monitor() {
                max_physical_size = monitor.size();
            }
            let max_size = max_physical_size.to_logical::<f64>(scale_factor);

            let mut ideal = solve_cache
                .ideal(true)
                .max(min_size)
                .as_physical()
                .to_logical(scale_factor);
            if ideal.width > max_size.width {
                ideal.width = max_size.width;
            }
            if ideal.height > max_size.height {
                ideal.height = max_size.height;
            }

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
            "Window::resume: constructed with physical size {size:?}, scale factor {scale_factor}",
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
        let mut surface = shared.instance.new_surface(window.clone(), transparent)?;
        shared.resume(&surface)?;
        surface.configure(&mut shared.draw.as_mut().unwrap().draw, size);

        let winit_id = window.id();

        #[cfg(feature = "accesskit")]
        let proxy = shared.proxy.0.clone();
        #[cfg(feature = "accesskit")]
        let accesskit = accesskit_winit::Adapter::with_event_loop_proxy(el, &window, proxy);

        self.window = Some(WindowData {
            window,
            #[cfg(all(wayland_platform, feature = "clipboard"))]
            wayland_clipboard,
            surface,
            frame_count: (Instant::now(), 0),

            #[cfg(feature = "accesskit")]
            accesskit,

            window_id: self.ev_state.window_id,
            solve_cache,
            theme_window,
            need_redraw: true,
        });

        // TODO: construct accesskit adapter

        self.apply_size(data, true);

        log::trace!(target: "kas_perf::wgpu::window", "resume: {}µs", time.elapsed().as_micros());
        Ok(winit_id)
    }

    /// Close (suspend) the window, keeping state (widget)
    ///
    /// Returns `true` unless this `Window` should be destoyed.
    pub(super) fn suspend(&mut self, shared: &mut Shared<A, G, T>, data: &A) -> bool {
        if let Some(ref mut window) = self.window {
            self.ev_state.suspended(shared);

            let action = self
                .ev_state
                .flush_pending(shared, window, self.widget.as_node(data));

            self.window = None;
            !action.contains(Action::CLOSE)
        } else {
            true
        }
    }

    /// Handle an event
    ///
    /// Returns `true` to force polling temporarily.
    pub(super) fn handle_event(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
        event: WindowEvent,
    ) -> bool {
        let Some(ref mut window) = self.window else {
            return false;
        };

        #[cfg(feature = "accesskit")]
        window.accesskit.process_event(&window.window, &event);

        match event {
            WindowEvent::Moved(_) | WindowEvent::Destroyed => false,
            WindowEvent::Resized(size) => {
                if window
                    .surface
                    .configure(&mut shared.draw.as_mut().unwrap().draw, size.cast())
                {
                    self.apply_size(data, false);
                }
                false
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // This event is generated when constructing a window but already handled
                if scale_factor as f32 == self.ev_state.config.scale_factor() {
                    return false;
                }

                // Note: API allows us to set new window size here.
                self.ev_state.update_config(scale_factor as f32);

                let config = self.ev_state.config();
                shared.theme.update_window(&mut window.theme_window, config);

                // NOTE: we could try resizing here in case the window is too
                // small due to non-linear scaling, but it appears unnecessary.
                window.solve_cache.invalidate_rule_cache();

                // Force a reconfigure to update text objects:
                self.reconfigure(data);

                false
            }
            WindowEvent::RedrawRequested => self.do_draw(shared, data).is_err(),
            event => {
                self.ev_state.with(shared, window, |cx| {
                    cx.handle_winit(&mut self.widget, data, event);
                });
                false
            }
        }
    }

    /// Handle all pending items before event loop sleeps
    pub(super) fn flush_pending(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
    ) -> (Action, Option<Instant>) {
        let Some(ref window) = self.window else {
            return (Action::empty(), None);
        };

        let action = self
            .ev_state
            .flush_pending(shared, window, self.widget.as_node(data));

        if action.contains(Action::CLOSE) {
            return (action, None);
        }
        self.handle_action(shared, data, action);

        let window = self.window.as_mut().unwrap();
        let resume = match (
            self.ev_state.next_resume(),
            window.surface.common_mut().next_resume(),
        ) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.min(b)),
        };

        // NOTE: need_frame_update() does not imply a need to redraw, but other
        // approaches do not yield good frame timing for e.g. kinetic scrolling.
        if window.need_redraw || self.ev_state.need_frame_update() {
            window.request_redraw();
        }

        (action, resume)
    }

    /// Send an erased message
    pub(super) fn send_erased(&mut self, id: Id, msg: Erased) {
        self.ev_state.send_erased(id, msg);
    }

    /// Handle an action (excludes handling of CLOSE and EXIT)
    pub(super) fn handle_action(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
        mut action: Action,
    ) {
        if action.contains(Action::EVENT_CONFIG)
            && let Some(ref mut window) = self.window
        {
            self.ev_state.update_config(window.scale_factor() as f32);
            action |= Action::UPDATE;
        }
        if action.contains(Action::UPDATE) {
            self.update(data);
        }
        if action.contains(Action::THEME_SWITCH) {
            if let Some(ref mut window) = self.window {
                let config = self.ev_state.config();
                window.theme_window = shared.theme.new_window(config);
            }
            action |= Action::RESIZE;
        } else if action.contains(Action::THEME_UPDATE) {
            if let Some(ref mut window) = self.window {
                let config = self.ev_state.config();
                shared.theme.update_window(&mut window.theme_window, config);
            }
            action |= Action::RESIZE;
        }
        if action.contains(Action::RESIZE) {
            if let Some(ref mut window) = self.window {
                window.solve_cache.invalidate_rule_cache();
            }
            self.apply_size(data, false);
        } else if !(action & (Action::SET_RECT | Action::SCROLLED)).is_empty() {
            self.apply_size(data, false);
        }
        debug_assert!(!action.contains(Action::REGION_MOVED));
        if !action.is_empty()
            && let Some(ref mut window) = self.window
        {
            window.need_redraw = true;
        }
    }

    #[cfg(feature = "accesskit")]
    pub(super) fn accesskit_event(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
        event: accesskit_winit::WindowEvent,
    ) {
        let Some(ref mut window) = self.window else {
            return;
        };

        use accesskit_winit::WindowEvent as WE;
        match event {
            WE::InitialTreeRequested => window
                .accesskit
                .update_if_active(|| self.ev_state.accesskit_tree_update(&self.widget)),
            WE::ActionRequested(request) => {
                self.ev_state.with(shared, window, |cx| {
                    cx.handle_accesskit_action(self.widget.as_node(data), request);
                });
            }
            WE::AccessibilityDeactivated => {
                self.ev_state.disable_accesskit();
            }
        }
    }

    pub(super) fn update_timer(
        &mut self,
        shared: &mut Shared<A, G, T>,
        data: &A,
        requested_resume: Instant,
    ) {
        let Some(ref mut window) = self.window else {
            return;
        };

        if window.surface.common_mut().immediate_redraw() {
            window.need_redraw = true;
            window.request_redraw();
        }

        let widget = self.widget.as_node(data);

        if Some(requested_resume) == self.ev_state.next_resume() {
            self.ev_state.with(shared, window, |cx| {
                cx.update_timer(widget);
            });
        } else {
            #[allow(clippy::drop_non_drop)]
            drop(widget); // make the borrow checker happy
        }
    }

    /// Add or reposition a pop-up
    pub(super) fn add_popup(&mut self, data: &A, id: WindowId, popup: PopupDescriptor) {
        let Some(ref window) = self.window else {
            return;
        };

        let size = window.theme_window.size();
        let mut cx = ConfigCx::new(&size, &mut self.ev_state);
        self.widget.add_popup(&mut cx, data, id, popup);
    }

    pub(super) fn send_action(&mut self, action: Action) {
        self.ev_state.action(self.widget.id(), action);
    }

    pub(super) fn send_close(&mut self, id: WindowId) {
        if id == self.ev_state.window_id {
            self.ev_state.action(self.widget.id(), Action::CLOSE);
        } else if let Some(window) = self.window.as_ref() {
            let widget = &mut self.widget;
            let size = window.theme_window.size();
            let mut cx = ConfigCx::new(&size, &mut self.ev_state);
            widget.remove_popup(&mut cx, id);
        }
    }
}

// Internal functions
impl<A: AppData, G: GraphicsInstance, T: Theme<G::Shared>> Window<A, G, T> {
    fn reconfigure(&mut self, data: &A) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        self.ev_state
            .full_configure(window.theme_window.size(), self.widget.as_node(data));

        log::trace!(target: "kas_perf::wgpu::window", "reconfigure: {}µs", time.elapsed().as_micros());
    }

    fn update(&mut self, data: &A) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };

        let size = window.theme_window.size();
        let mut cx = ConfigCx::new(&size, &mut self.ev_state);
        cx.update(self.widget.as_node(data));

        log::trace!(target: "kas_perf::wgpu::window", "update: {}µs", time.elapsed().as_micros());
    }

    fn apply_size(&mut self, data: &A, first: bool) {
        let time = Instant::now();
        let Some(ref mut window) = self.window else {
            return;
        };
        let rect = Rect::new(Coord::ZERO, window.surface.size());
        log::debug!("apply_size: rect={rect:?}");

        let solve_cache = &mut window.solve_cache;
        let mut cx = ConfigCx::new(window.theme_window.size(), &mut self.ev_state);
        solve_cache.apply_rect(self.widget.as_node(data), &mut cx, rect, true);
        if first {
            solve_cache.print_widget_heirarchy(self.widget.as_tile());
        }
        self.widget.resize_popups(&mut cx, data);

        // Size restrictions may have changed due to content or size (line wrapping)
        let (restrict_min, restrict_max) = self.widget.properties().restrictions();
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
    pub(super) fn do_draw(&mut self, shared: &mut Shared<A, G, T>, data: &A) -> Result<(), ()> {
        let start = Instant::now();
        let Some(ref mut window) = self.window else {
            return Ok(());
        };

        let widget = self.widget.as_node(data);
        self.ev_state.with(shared, window, |cx| {
            cx.frame_update(widget);
        });

        #[cfg(feature = "accesskit")]
        if self.ev_state.accesskit_is_enabled() {
            window
                .accesskit
                .update_if_active(|| self.ev_state.accesskit_tree_update(&self.widget))
        }

        self.ev_state.clear_access_key_bindings();

        {
            let rect = Rect::new(Coord::ZERO, window.surface.size());
            let draw = window.surface.draw_iface(shared.draw.as_mut().unwrap());

            let mut draw = shared
                .theme
                .draw(draw, &mut self.ev_state, &mut window.theme_window);
            let draw_cx = DrawCx::new(&mut draw, self.widget.id());
            self.widget.draw(draw_cx);

            draw.new_pass(
                rect,
                Offset::ZERO,
                PassType::Clip,
                Box::new(|draw: &mut dyn ThemeDraw| draw.event_state_overlay()),
            );
        }
        let time2 = Instant::now();

        window.need_redraw = window.surface.common_mut().immediate_redraw();
        self.ev_state.action -= Action::REDRAW;
        // NOTE: we used to return Err(()) if !action.is_empty() here, e.g. if a
        // widget requested a resize during draw. Likely it's better not to do
        // this even if the frame is imperfect.

        let clear_color = if self.widget.properties().transparent() {
            Rgba::TRANSPARENT
        } else {
            shared.theme.clear_color()
        };
        let time3 = window
            .surface
            .present(&mut shared.draw.as_mut().unwrap().draw, clear_color);

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
        if window.frame_count.0 + SECOND <= end {
            log::debug!(
                "Window {:?}: {} frames in last second",
                window.window_id,
                window.frame_count.1
            );
            window.frame_count.0 = end;
            window.frame_count.1 = 0;
        }

        Ok(())
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

    /// Enable / disable IME and set purpose
    fn set_ime_allowed(&self, purpose: Option<ImePurpose>);

    /// Set IME cursor area
    fn set_ime_cursor_area(&self, rect: Rect);

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    fn winit_window(&self) -> Option<&winit::window::Window>;
}

impl<G: GraphicsInstance, T: Theme<G::Shared>> WindowDataErased for WindowData<G, T> {
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

    fn set_ime_allowed(&self, purpose: Option<ImePurpose>) {
        self.window.set_ime_allowed(purpose.is_some());
        if let Some(purpose) = purpose {
            self.window.set_ime_purpose(purpose);
        }
    }

    fn set_ime_cursor_area(&self, rect: Rect) {
        self.window
            .set_ime_cursor_area(rect.pos.as_physical(), rect.size.as_physical());
    }

    #[inline]
    fn winit_window(&self) -> Option<&winit::window::Window> {
        Some(&self.window)
    }
}
