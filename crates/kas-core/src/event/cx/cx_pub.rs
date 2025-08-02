// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use std::time::Duration;

use super::*;
use crate::HasId;
use crate::cast::Conv;
use crate::config::ConfigMsg;
use crate::draw::DrawShared;
use crate::geom::{Offset, Vec2};
use crate::theme::SizeCx;
#[cfg(all(wayland_platform, feature = "clipboard"))]
use crate::util::warn_about_error;
use crate::window::{PopupDescriptor, Window};
#[allow(unused)] use crate::{Events, Layout, Tile}; // for doc-links

impl EventState {
    /// Get the platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// True when the window has focus
    #[inline]
    pub fn window_has_focus(&self) -> bool {
        self.window_has_focus
    }

    /// True if [AccessKit](https://accesskit.dev/) is enabled
    #[cfg(feature = "accesskit")]
    #[inline]
    pub(crate) fn accesskit_is_enabled(&self) -> bool {
        self.accesskit_is_enabled
    }

    /// Check whether a widget is disabled
    ///
    /// A widget is disabled if any ancestor is.
    #[inline]
    pub fn is_disabled(&self, w_id: &Id) -> bool {
        // TODO(opt): we should be able to use binary search here
        for id in &self.disabled {
            if id.is_ancestor_of(w_id) {
                return true;
            }
        }
        false
    }

    /// Access event-handling configuration
    #[inline]
    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Is mouse panning enabled?
    #[inline]
    pub fn config_enable_pan(&self, source: PressSource) -> bool {
        source.is_touch()
            || source.is_primary()
                && self
                    .config
                    .event()
                    .mouse_pan()
                    .is_enabled_with(self.modifiers())
    }

    /// Is mouse text panning enabled?
    #[inline]
    pub fn config_enable_mouse_text_pan(&self) -> bool {
        self.config
            .event()
            .mouse_text_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Test pan threshold against config, adjusted for scale factor
    ///
    /// Returns true when `dist` is large enough to switch to pan mode.
    #[inline]
    pub fn config_test_pan_thresh(&self, dist: Offset) -> bool {
        Vec2::conv(dist).abs().max_comp() >= self.config.event().pan_dist_thresh()
    }

    /// Update event configuration
    #[inline]
    pub fn change_config(&mut self, msg: ConfigMsg) {
        self.action |= self.config.change_config(msg);
    }

    /// Set/unset a widget as disabled
    ///
    /// Disabled status applies to all descendants and blocks reception of
    /// events ([`Unused`] is returned automatically when the
    /// recipient or any ancestor is disabled).
    ///
    /// Disabling a widget clears navigation, selection and key focus when the
    /// target is disabled, and also cancels press/pan grabs.
    pub fn set_disabled(&mut self, target: Id, disable: bool) {
        if disable {
            self.cancel_event_focus(&target);
        }

        for (i, id) in self.disabled.iter().enumerate() {
            if target == id {
                if !disable {
                    self.redraw(target);
                    self.disabled.remove(i);
                }
                return;
            }
        }
        if disable {
            self.action(&target, Action::REDRAW);
            self.disabled.push(target);
        }
    }

    /// Schedule a timed update
    ///
    /// Widget updates may be used for delayed action. For animation, prefer to
    /// use [`Draw::animate`](crate::draw::Draw::animate) or
    /// [`Self::request_frame_timer`].
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid, except from an
    /// [`Event::Timer`] handler (where it may cause an infinite loop).
    ///
    /// Multiple timer requests with the same `id` and `handle` are merged
    /// (see [`TimerHandle`] documentation).
    pub fn request_timer(&mut self, id: Id, handle: TimerHandle, delay: Duration) {
        let time = Instant::now() + delay;
        if let Some(row) = self
            .time_updates
            .iter_mut()
            .find(|row| row.1 == id && row.2 == handle)
        {
            let earliest = handle.earliest();
            if earliest && row.0 <= time || !earliest && row.0 >= time {
                return;
            }

            row.0 = time;
        } else {
            log::trace!(
                target: "kas_core::event",
                "request_timer: update {id} at now+{}ms",
                delay.as_millis()
            );
            self.time_updates.push((time, id, handle));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    /// Schedule a frame timer update
    ///
    /// Widget `id` will receive [`Event::Timer`] with this `handle` either
    /// before or soon after the next frame is drawn.
    ///
    /// This may be useful for animations which mutate widget state. Animations
    /// which don't mutate widget state may use
    /// [`Draw::animate`](crate::draw::Draw::animate) instead.
    ///
    /// It is expected that `handle.earliest() == true` (style guide).
    pub fn request_frame_timer(&mut self, id: Id, handle: TimerHandle) {
        debug_assert!(handle.earliest());
        self.frame_updates.insert((id, handle));
    }

    /// Notify that a widget must be redrawn
    ///
    /// This is equivalent to calling [`Self::action`] with [`Action::REDRAW`].
    #[inline]
    pub fn redraw(&mut self, id: impl HasId) {
        self.action(id, Action::REDRAW);
    }

    /// Notify that a widget must be resized
    ///
    /// This is equivalent to calling [`Self::action`] with [`Action::RESIZE`].
    #[inline]
    pub fn resize(&mut self, id: impl HasId) {
        self.action(id, Action::RESIZE);
    }

    /// Notify that widgets under self may have moved
    #[inline]
    pub fn region_moved(&mut self) {
        // Do not take id: this always applies to the whole window
        self.action |= Action::REGION_MOVED;
    }

    /// Terminate the GUI
    #[inline]
    pub fn exit(&mut self) {
        self.send(Id::ROOT, Command::Exit);
    }

    /// Notify that an [`Action`] should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn action(&mut self, id: impl HasId, action: Action) {
        fn inner(cx: &mut EventState, id: Id, mut action: Action) {
            if action.contains(Action::UPDATE) {
                cx.request_update(id);
                action.remove(Action::UPDATE);
            }

            // TODO(opt): handle sub-tree SCROLLED. This is probably equivalent to using `_replay` without a message but with `scroll = Scroll::Scrolled`.
            // TODO(opt): handle sub-tree SET_RECT and RESIZE.
            // NOTE: our draw system is incompatible with partial redraws, and
            // in any case redrawing is extremely fast.

            cx.action |= action;
        }
        inner(self, id.has_id(), action)
    }

    /// Pass an [action](Self::action) given some `id`
    #[inline]
    pub(crate) fn opt_action(&mut self, id: Option<Id>, action: Action) {
        if let Some(id) = id {
            self.action(id, action);
        }
    }

    /// Notify that an [`Action`] should happen for the whole window
    ///
    /// Using [`Self::action`] with a widget `id` instead of this method is
    /// potentially more efficient (supports future optimisations), but not
    /// always possible.
    #[inline]
    pub fn window_action(&mut self, action: Action) {
        self.action |= action;
    }

    /// Request update to widget `id`
    ///
    /// This will call [`Events::update`] on `id`.
    pub fn request_update(&mut self, id: Id) {
        self.pending_update = if let Some(id2) = self.pending_update.take() {
            Some(id.common_ancestor(&id2))
        } else {
            Some(id)
        };
    }
}

impl<'a> EventCx<'a> {
    /// Configure a widget
    ///
    /// Note that, when handling events, this method returns the *old* state.
    ///
    /// This is a shortcut to [`ConfigCx::configure`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: Id) {
        widget._configure(&mut self.config_cx(), id);
    }

    /// Update a widget
    ///
    /// [`Events::update`] will be called recursively on each child and finally
    /// `self`. If a widget stores state which it passes to children as input
    /// data, it should call this (or [`ConfigCx::update`]) after mutating the state.
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        widget._update(&mut self.config_cx());
    }

    /// Add a pop-up
    ///
    /// A pop-up is a box used for things like tool-tips and menus which is
    /// drawn on top of other content and has focus for input.
    ///
    /// Depending on the host environment, the pop-up may be a special type of
    /// window without borders and with precise placement, or may be a layer
    /// drawn in an existing window.
    ///
    /// The popup automatically receives mouse-motion events
    /// ([`Event::CursorMove`]) which may be used to navigate menus.
    /// The parent automatically receives the "depressed" visual state.
    ///
    /// It is recommended to call [`EventState::set_nav_focus`] or
    /// [`EventState::next_nav_focus`] after this method.
    ///
    /// A pop-up may be closed by calling [`EventCx::close_window`] with
    /// the [`WindowId`] returned by this method.
    pub(crate) fn add_popup(&mut self, popup: PopupDescriptor, set_focus: bool) -> WindowId {
        log::trace!(target: "kas_core::event", "add_popup: {popup:?}");

        let parent_id = self.window.window_id();
        let id = self.runner.add_popup(parent_id, popup.clone());
        let mut old_nav_focus = None;
        if set_focus {
            old_nav_focus = self.nav_focus.clone();
            self.clear_nav_focus();
        }
        self.popups.push(PopupState {
            id,
            desc: popup,
            old_nav_focus,
            is_sized: false,
        });
        id
    }

    /// Resize and reposition an existing pop-up
    ///
    /// This method takes a new [`PopupDescriptor`]. Its first field, `id`, is
    /// expected to remain unchanged but other fields may differ.
    pub(crate) fn reposition_popup(&mut self, id: WindowId, desc: PopupDescriptor) {
        self.runner.reposition_popup(id, desc.clone());
        for popup in self.popups.iter_mut() {
            if popup.id == id {
                debug_assert_eq!(popup.desc.id, desc.id);
                popup.desc = desc;
                break;
            }
        }
    }

    /// Add a window
    ///
    /// Typically an application adds at least one window before the event-loop
    /// starts (see `kas_wgpu::Toolkit::add`), however that method is not
    /// available to a running UI. This method may be used instead.
    ///
    /// Requirement: the type `Data` must match the type of data passed to the
    /// [`Runner`](https://docs.rs/kas/latest/kas/runner/struct.Runner.html)
    /// and used by other windows. If not, a run-time error will result.
    ///
    /// Caveat: if an error occurs opening the new window it will not be
    /// reported (except via log messages).
    #[inline]
    pub fn add_window<Data: 'static>(&mut self, window: Window<Data>) -> WindowId {
        let data_type_id = std::any::TypeId::of::<Data>();
        unsafe {
            let window: Window<()> = std::mem::transmute(window);
            self.runner.add_window(window, data_type_id)
        }
    }

    /// Close a window or pop-up
    ///
    /// Navigation focus will return to whichever widget had focus before
    /// the popup was open.
    pub fn close_window(&mut self, mut id: WindowId) {
        for (index, p) in self.popups.iter().enumerate() {
            if p.id == id {
                id = self.close_popup(index);
                break;
            }
        }

        self.runner.close_window(id);
    }

    /// Enable window dragging for current click
    ///
    /// This calls [`winit::window::Window::drag_window`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.drag_window). Errors are ignored.
    pub fn drag_window(&self) {
        if let Some(ww) = self.window.winit_window()
            && let Err(e) = ww.drag_window()
        {
            log::warn!("EventCx::drag_window: {e}");
        }
    }

    /// Enable window resizing for the current click
    ///
    /// This calls [`winit::window::Window::drag_resize_window`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.drag_resize_window). Errors are ignored.
    pub fn drag_resize_window(&self, direction: ResizeDirection) {
        if let Some(ww) = self.window.winit_window()
            && let Err(e) = ww.drag_resize_window(direction)
        {
            log::warn!("EventCx::drag_resize_window: {e}");
        }
    }

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    pub fn get_clipboard(&mut self) -> Option<String> {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            return match cb.load() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn_about_error("Failed to get clipboard contents", &e);
                    None
                }
            };
        }

        self.runner.get_clipboard()
    }

    /// Attempt to set clipboard contents
    pub fn set_clipboard(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            cb.store(content);
            return;
        }

        self.runner.set_clipboard(content)
    }

    /// True if the primary buffer is enabled
    #[inline]
    pub fn has_primary(&self) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(unix)] {
                true
            } else {
                false
            }
        }
    }

    /// Get contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    pub fn get_primary(&mut self) -> Option<String> {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            return match cb.load_primary() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn_about_error("Failed to get clipboard contents", &e);
                    None
                }
            };
        }

        self.runner.get_primary()
    }

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    pub fn set_primary(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            cb.store_primary(content);
            return;
        }

        self.runner.set_primary(content)
    }

    /// Get a [`SizeCx`]
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Events::configure`].
    pub fn size_cx(&self) -> SizeCx<'_> {
        SizeCx::new(self.window.theme_size())
    }

    /// Get a [`ConfigCx`]
    pub fn config_cx(&mut self) -> ConfigCx<'_> {
        let size = self.window.theme_size();
        ConfigCx::new(size, self.state)
    }

    /// Get a [`DrawShared`]
    pub fn draw_shared(&mut self) -> &mut dyn DrawShared {
        self.runner.draw_shared()
    }

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    pub fn winit_window(&self) -> Option<&winit::window::Window> {
        self.window.winit_window()
    }
}
