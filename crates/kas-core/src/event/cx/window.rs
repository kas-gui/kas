// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event state: window management

use super::{EventCx, EventState, PopupState};
use crate::Id;
use crate::event::FocusSource;
use crate::runner::Platform;
use crate::util::warn_about_error;
use crate::window::{PopupDescriptor, Window, WindowId};
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
}
