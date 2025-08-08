// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event state: window management

use super::{EventCx, EventState, PopupState};
use crate::event::{Event, FocusSource};
use crate::runner::{MessageStack, Platform, RunnerT, WindowDataErased};
#[cfg(all(wayland_platform, feature = "clipboard"))]
use crate::util::warn_about_error;
use crate::window::{PopupDescriptor, Window, WindowId};
use crate::{Action, Id, Tile, Widget};
use winit::window::ResizeDirection;

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

    // Remove popup at index and return its [`WindowId`]
    //
    // Panics if `index` is out of bounds.
    //
    // The caller must call `runner.close_window(window_id)`.
    #[must_use]
    pub(super) fn close_popup(&mut self, index: usize) -> WindowId {
        let state = self.popups.remove(index);
        if state.is_sized {
            self.popup_removed.push((state.desc.id, state.id));
        }
        self.mouse.tooltip_popup_close(&state.desc.parent);

        if let Some(id) = state.old_nav_focus {
            self.set_nav_focus(id, FocusSource::Synthetic);
        }

        state.id
    }

    pub(crate) fn confirm_popup_is_sized(&mut self, id: WindowId) {
        for popup in &mut self.popups {
            if popup.id == id {
                popup.is_sized = true;
            }
        }
    }

    /// Handle all pending items before event loop sleeps
    pub(crate) fn flush_pending<'a, A>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        window: &'a dyn WindowDataErased,
        messages: &'a mut MessageStack,
        win: &mut Window<A>,
        data: &A,
    ) -> Action {
        self.with(runner, window, messages, |cx| {
            while let Some((id, wid)) = cx.popup_removed.pop() {
                cx.send_event(win.as_node(data), id, Event::PopupClosed(wid));
            }

            cx.mouse_handle_pending(win, data);
            cx.touch_handle_pending(win, data);

            if let Some(id) = cx.pending_update.take() {
                win.as_node(data).find_node(&id, |node| cx.update(node));
            }

            if cx.pending_nav_focus.is_some() {
                cx.handle_pending_nav_focus(win.as_node(data));
            }

            // Update sel focus after nav focus:
            if let Some(pending) = cx.pending_sel_focus.take() {
                cx.set_sel_focus(cx.window, win.as_node(data), pending);
            }

            while let Some((id, msg)) = cx.send_queue.pop_front() {
                cx.send_or_replay(win.as_node(data), id, msg);
            }

            // Poll futures. TODO(opt): this does not need to happen so often,
            // but just in frame_update is insufficient.
            cx.poll_futures(win.as_node(data));

            // Finally, clear the region_moved flag (mouse and touch sub-systems handle this).
            if cx.action.contains(Action::REGION_MOVED) {
                cx.action.remove(Action::REGION_MOVED);
                cx.action.insert(Action::REDRAW);
            }
        });

        if let Some(icon) = self.mouse.update_cursor_icon() {
            window.set_cursor_icon(icon);
        }

        std::mem::take(&mut self.action)
    }

    /// Window has been closed: clean up state
    pub(crate) fn suspended(&mut self, runner: &mut dyn RunnerT) {
        while !self.popups.is_empty() {
            let id = self.close_popup(self.popups.len() - 1);
            runner.close_window(id);
        }
    }
}

impl<'a> EventCx<'a> {
    // Closes any popup which is not an ancestor of `id`
    pub(super) fn close_non_ancestors_of(&mut self, id: Option<&Id>) {
        for index in (0..self.popups.len()).rev() {
            if let Some(id) = id
                && self.popups[index].desc.id.is_ancestor_of(id)
            {
                continue;
            }

            let id = self.close_popup(index);
            self.runner.close_window(id);
        }
    }

    pub(super) fn handle_close(&mut self) {
        let mut id = self.window_id;
        if !self.popups.is_empty() {
            let index = self.popups.len() - 1;
            id = self.close_popup(index);
        }
        self.runner.close_window(id);
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

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    pub fn winit_window(&self) -> Option<&winit::window::Window> {
        self.window.winit_window()
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not handled, since for these
    /// events the graphics backend must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    pub(crate) fn handle_winit<A>(
        &mut self,
        win: &mut Window<A>,
        data: &A,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent::*;

        match event {
            CloseRequested => self.action(win.id(), Action::CLOSE),
            /* Not yet supported: see #98
            DroppedFile(path) => ,
            HoveredFile(path) => ,
            HoveredFileCancelled => ,
            */
            Focused(state) => {
                self.window_has_focus = state;
                if state {
                    // Required to restart theme animations
                    self.redraw(win.id());
                } else {
                    // Window focus lost: close all popups
                    while let Some(id) = self.popups.last().map(|state| state.id) {
                        self.close_window(id);
                    }
                }
            }
            KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.keyboard_input(win.as_node(data), event, is_synthetic),
            ModifiersChanged(modifiers) => self.modifiers_changed(modifiers.state()),
            Ime(event) => self.ime_event(win.as_node(data), event),
            CursorMoved { position, .. } => self.handle_cursor_moved(win, data, position.into()),
            CursorEntered { .. } => self.handle_cursor_entered(),
            CursorLeft { .. } => self.handle_cursor_left(win.as_node(data)),
            MouseWheel { delta, .. } => self.handle_mouse_wheel(win.as_node(data), delta),
            MouseInput { state, button, .. } => {
                self.handle_mouse_input(win.as_node(data), state, button)
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            Touch(touch) => self.handle_touch_event(win, data, touch),
            _ => (),
        }
    }
}
