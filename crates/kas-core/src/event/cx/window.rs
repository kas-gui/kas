// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event state: window management

use super::{EventCx, EventState, PopupState, TopWindow};
use crate::cast::Cast;
use crate::data_transfer::PlainUtf8Transfer;
use crate::event::cx::handle::CurrentWindow;
use crate::event::{Event, FocusSource};
use crate::geom::DVec2;
use crate::messages::Erased;
use crate::runner::{AppData, Platform, RunnerT, WindowDataErased};
use crate::theme::ThemeSize;
use crate::window::{PopupDescriptor, Properties, WindowId};
use crate::{Id, Node, Widget, WindowActions};
use linear_map::LinearMap;
use std::sync::Arc;
use winit::data_transfer::{
    DataTransfer, DataTransferId, DataTransferSend, TransferType, TypeHint, TypedData,
};
use winit::event::{ElementState, FingerId, Ime, KeyEvent, MouseButton, MouseScrollDelta};
use winit::event_loop::{ActiveEventLoop, AsyncRequestSerial};
use winit::keyboard::ModifiersState;

pub(crate) enum InputEvent {
    Focused(bool),
    KeyboardInput {
        event: KeyEvent,
        is_synthetic: bool,
    },
    ModifiersChanged(ModifiersState),
    Ime(Ime),
    MouseMoved {
        position: DVec2,
    },
    MouseEntered,
    MouseLeft,
    MouseWheel {
        delta: MouseScrollDelta,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
    TouchStart {
        finger_id: FingerId,
        position: DVec2,
    },
    TouchMoved {
        finger_id: FingerId,
        position: DVec2,
    },
    TouchEnd {
        finger_id: FingerId,
        position: DVec2,
    },
    /// This is a copy of [`winit::event::WindowEvent::DataTransferReceived`].
    DataTransferReceived {
        id: DataTransferId,
        serial: AsyncRequestSerial,
        value: Arc<dyn TypedData>,
    },
}

/// Control over data transfer type selection
///
/// Multiple types may be selected. No transfer happens if no types are selected.
pub struct TypeSelector<'a> {
    el: &'a dyn ActiveEventLoop,
    dt: &'a dyn DataTransfer,
    transfers: &'a mut LinearMap<AsyncRequestSerial, Id>,
    id: DataTransferId,
    target: Id,
}
impl<'a> TypeSelector<'a> {
    /// Return the transfer identifier
    pub fn id(&self) -> DataTransferId {
        self.id
    }

    /// Get available types
    pub fn types(&self) -> &dyn DataTransfer {
        self.dt
    }

    /// Elect to receive data of this type
    pub fn select(&mut self, type_: &(dyn TransferType + 'static)) {
        match self.el.fetch_data_transfer(self.id, type_) {
            Ok(serial) => {
                self.transfers.insert(serial, self.target.clone());
            }
            Err(e) => log::warn!("get_clipboard failed: {e}"),
        }
    }
}

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
    pub(super) fn close_popup(&mut self, runner: &mut dyn RunnerT, index: usize) {
        let state = self.popups.remove(index);
        runner.send_erased(state.desc.id, Erased::new(Event::PopupClosed(state.id)));
        self.mouse.tooltip_popup_close(&state.desc.parent);

        if let Some(id) = state.old_nav_focus {
            self.set_nav_focus(id, FocusSource::Synthetic);
        }

        runner.close_window(state.id);
    }

    /// Handle all pending items before event loop sleeps
    #[must_use]
    pub(crate) fn flush_pending<'a>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        theme: &'a dyn ThemeSize,
        window: &'a dyn WindowDataErased,
        mut node: Node,
        mut moved: bool,
    ) -> WindowActions {
        if !self.pending_send_targets.is_empty() {
            runner.set_send_targets(&mut self.pending_send_targets);
        }

        let actions = self.with(runner, theme, window, |cx| {
            if cx.pending.is_some() {
                cx.handle_pending_event(node.re());
            }

            cx.mouse_handle_pending(node.re());
            cx.touch_handle_pending(node.re());

            if cx.nav.has_pending_changes() {
                cx.handle_pending_nav_focus(node.re());
            }

            // Update sel focus after nav focus:
            if cx.input.has_pending_changes() {
                cx.flush_pending_input_focus(node.re());
            }

            // Poll futures; these may push messages to cx.send_queue.
            cx.poll_futures();

            let window_id = Id::ROOT.make_child(cx.window_id.get().cast());
            while let Some((id, msg)) = cx.send_queue.pop_front() {
                if !id.is_valid() {
                    cx.push_erased(msg);
                } else if window_id.is_ancestor_of(&id) {
                    cx.send_or_replay(node.re(), id, msg);
                } else {
                    cx.runner.send_erased(id, msg);
                }
            }

            if cx.actions == WindowActions::Moved {
                moved = true;
                cx.actions = WindowActions::Redraw;
            }
            if moved {
                cx.touch_handle_moved(node.as_tile());
                cx.mouse_handle_moved(node);
            }
        });

        if let Some(icon) = self.mouse.update_pointer_icon() {
            window.set_pointer_icon(icon);
        }

        actions.into()
    }

    pub(super) fn close_all_popups(&mut self, runner: &mut dyn RunnerT) {
        while !self.popups.is_empty() {
            self.close_popup(runner, self.popups.len() - 1);
        }
    }

    /// Application suspended. Clean up temporary state.
    pub(crate) fn suspended(&mut self, runner: &mut dyn RunnerT) {
        self.close_all_popups(runner);
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

            self.cx.state.close_popup(self.runner, index);
        }
    }

    pub(super) fn handle_close(&mut self) {
        if !self.popups.is_empty() {
            let index = self.popups.len() - 1;
            self.cx.state.close_popup(self.runner, index);
            return;
        }

        self.runner.close_window(self.window_id);
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
    /// ([`Event::PointerMove`]) which may be used to navigate menus.
    /// The parent automatically receives the "depressed" visual state.
    ///
    /// It is recommended to call [`EventState::set_nav_focus`] or
    /// [`EventState::next_nav_focus`] after this method.
    ///
    /// A pop-up may be closed by calling [`EventCx::close_window`] with
    /// the [`WindowId`] returned by this method.
    pub(crate) fn add_popup(&mut self, popup: PopupDescriptor) -> WindowId {
        log::trace!(target: "kas_core::event", "add_popup: {popup:?}");

        let parent_id = self.window_id;
        let id = self.runner.add_popup(parent_id, popup.clone());
        let mut old_nav_focus = None;
        if popup.takes_focus {
            old_nav_focus = self.nav_focus().cloned();
            self.clear_nav_focus();
        }
        self.popups.push(PopupState {
            id,
            desc: popup,
            old_nav_focus,
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

    /// Add a data-less window
    ///
    /// This method may be used to attach a new window to the UI at run-time.
    /// This method supports windows which do not take data; see also
    /// [`Self::add_window`].
    ///
    /// Adding the window is infallible. Opening the new window is theoretically
    /// fallible (unlikely assuming a window has already been opened).
    ///
    /// Sets [`properties.parent`](Properties::parent) to this window's
    /// [`WindowId`], then constructs a window over the given `widget` and
    /// `title`.
    #[inline]
    pub fn add_window(
        &mut self,
        widget: impl Widget<Data = ()> + 'static,
        title: impl ToString,
        properties: Properties,
    ) -> WindowId {
        self.add_window_boxed(Box::new(widget), title.to_string(), properties)
    }

    /// Add a data-less window
    ///
    /// This is a variant of [`Self::add_window`] taking an already-boxed widget.
    pub fn add_window_boxed(
        &mut self,
        widget: Box<dyn Widget<Data = ()>>,
        title: String,
        mut properties: Properties,
    ) -> WindowId {
        properties.parent = Some(self.window_id);
        self.runner.add_window(widget, title, properties)
    }

    /// Add a window able to access top-level app data
    ///
    /// This method may be used to attach a new window to the UI at run-time.
    /// See also [`Self::add_window`] for a variant which does not
    /// require a `Data` parameter.
    ///
    /// Requirement: the type `Data` must match the type of data passed to the
    /// [`Runner`](https://docs.rs/kas/latest/kas/runner/struct.Runner.html)
    /// and used by other windows. If not, a run-time error will result.
    ///
    /// Adding the window is infallible. Opening the new window is theoretically
    /// fallible (unlikely assuming a window has already been opened).
    ///
    /// Sets [`properties.parent`](Properties::parent) to this window's
    /// [`WindowId`], then constructs a window over the given `widget` and
    /// `title`.
    #[inline]
    pub fn add_window_with_data<Data: AppData>(
        &mut self,
        widget: Box<dyn Widget<Data = Data>>,
        title: String,
        mut properties: Properties,
    ) -> WindowId {
        properties.parent = Some(self.window_id);

        let data_type_id = std::any::TypeId::of::<Data>();
        unsafe {
            let widget: Box<dyn Widget<Data = ()>> = std::mem::transmute(widget);
            self.runner
                .add_window_with_data(widget, data_type_id, title.to_string(), properties)
        }
    }

    /// Close a window or pop-up
    ///
    /// Navigation focus will return to whichever widget had focus before
    /// the popup was open.
    pub fn close_window(&mut self, id: WindowId) {
        for (index, p) in self.popups.iter().enumerate() {
            if p.id == id {
                self.cx.state.close_popup(self.runner, index);
                return;
            }
        }

        self.runner.close_window(id);
    }

    /// Request clipboard contents
    ///
    /// Assuming the clipboard is not empty, closure `type_selector` will be
    /// called to select a data type for transfer.
    ///
    /// On success, widget `target` will receive [`Event::DataTransfer`] with
    /// the given [`DataTransferId`]. On failure, no transfer event will be
    /// delivered.
    pub fn get_clipboard(&mut self, target: Id, mut type_selector: impl FnMut(&mut TypeSelector)) {
        let el = self.window.event_loop();

        let Ok(Some(id)) = el
            .clipboard()
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
        else {
            return;
        };

        let Ok(dt) = el
            .data_transfer(id)
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
        else {
            return;
        };

        type_selector(&mut TypeSelector {
            el,
            dt: &*dt,
            transfers: &mut self.data_transfers,
            id,
            target,
        });
    }

    /// Request clipboard contents as plain UTF-8
    ///
    /// On success, widget `target` will receive [`Event::DataTransfer`] with
    /// the given [`DataTransferId`]. On failure, no transfer event will be
    /// delivered.
    pub fn get_clipboard_plain_text(&mut self, target: Id) -> Option<DataTransferId> {
        let el = self.window.event_loop();

        let id = el
            .clipboard()
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
            .ok()
            .flatten()?;

        let serial = el
            .fetch_data_transfer(id, &TypeHint::Plaintext)
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
            .ok()?;

        self.data_transfers.insert(serial, target);
        Some(id)
    }

    /// Attempt to set clipboard contents
    pub fn set_clipboard(&mut self, content: Box<dyn DataTransferSend>) {
        match self.window.event_loop().set_clipboard(content) {
            Ok(()) => (),
            Err(e) => log::warn!("set_clipboard failed: {e}"),
        }
    }

    /// Attempt to set clipboard contents as plain text
    pub fn set_clipboard_plain_text(&mut self, content: String) {
        self.set_clipboard(Box::new(PlainUtf8Transfer::from(content)));
    }

    /// True if there is primary selection buffer (Linux)
    #[inline]
    pub fn has_primary(&self) -> bool {
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "android", target_os = "emscripten")),
        ))]
        {
            return self.window.has_primary_selection();
        }

        #[allow(unused)]
        {
            false
        }
    }

    /// Request contents of primary selection buffer
    ///
    /// Assuming the clipboard is not empty, closure `type_selector` will be
    /// called to select a data type for transfer.
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    ///
    /// On success, widget `target` will receive [`Event::DataTransfer`] with
    /// the given [`DataTransferId`]. On failure, no transfer event will be
    /// delivered.
    pub fn get_primary(&mut self, target: Id, mut type_selector: impl FnMut(&mut TypeSelector)) {
        let el = self.window.event_loop();

        let Some(Ok(Some(id))) = el.primary_selection_ext().map(|ext| {
            ext.primary_selection()
                .inspect_err(|e| log::warn!("get_primary failed: {e}"))
        }) else {
            return;
        };

        let Ok(dt) = el
            .data_transfer(id)
            .inspect_err(|e| log::warn!("get_primary failed: {e}"))
        else {
            return;
        };

        type_selector(&mut TypeSelector {
            el,
            dt: &*dt,
            transfers: &mut self.data_transfers,
            id,
            target,
        });
    }

    /// Request contents of primary selection buffer as plain UTF-8
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    ///
    /// On success, widget `target` will receive [`Event::DataTransfer`] with
    /// the given [`DataTransferId`]. On failure, no transfer event will be
    /// delivered.
    pub fn get_primary_plain_text(&mut self, target: Id) -> Option<DataTransferId> {
        let el = self.window.event_loop();

        let id = el
            .primary_selection_ext()?
            .primary_selection()
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
            .ok()
            .flatten()?;

        let serial = el
            .fetch_data_transfer(id, &TypeHint::Plaintext)
            .inspect_err(|e| log::warn!("get_clipboard failed: {e}"))
            .ok()?;

        self.data_transfers.insert(serial, target);
        Some(id)
    }

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    pub fn set_primary(&mut self, content: Box<dyn DataTransferSend>) {
        if let Some(pri) = self.window.event_loop().primary_selection_ext() {
            match pri.set_primary_selection(content) {
                Ok(()) => (),
                Err(e) => log::warn!("set_primary failed: {e}"),
            }
        }
    }

    /// Set contents of primary buffer as plain text
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    pub fn set_primary_plain_text(&mut self, content: String) {
        if self.has_primary() {
            self.set_primary(Box::new(PlainUtf8Transfer::from(content)));
        }
    }

    /// Get a handle to the current window
    ///
    /// This returns a handle to the current popup or top-level window.
    #[inline]
    pub fn current_window(&mut self) -> CurrentWindow<'_> {
        let (id, window) = self.window.current_window();
        CurrentWindow {
            id,
            runner: self.runner,
            window,
        }
    }

    /// Get a handle to the top (root) window
    ///
    /// A top (root) window is one which does not have a parent. An application
    /// may have multiple top (root) windows. Child windows (e.g. modal windows
    /// and popups) windows each have a parent window (which may also be a child
    /// window); in all cases there should be a top (root ancestor) window.
    #[inline]
    pub fn top_window(&mut self) -> TopWindow<'_, 'a> {
        TopWindow {
            runner: self.runner,
            window: self.window.top_window(),
            cx: &mut self.cx,
        }
    }

    /// Handle an input event
    pub(crate) fn input_event(&mut self, node: Node, event: InputEvent) {
        if let Some(pending) = self.pending.as_mut() {
            // Merge high-frequency events to avoid a backlog
            match (&event, pending) {
                (
                    InputEvent::MouseMoved { position },
                    InputEvent::MouseMoved { position: pos2 },
                ) => {
                    *pos2 = *position;
                    return;
                }
                (
                    InputEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(x, y),
                    },
                    InputEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(x2, y2),
                    },
                ) => {
                    *x2 += x;
                    *y2 += y;
                    return;
                }
                (
                    InputEvent::MouseWheel {
                        delta: MouseScrollDelta::PixelDelta(d),
                    },
                    InputEvent::MouseWheel {
                        delta: MouseScrollDelta::PixelDelta(d2),
                    },
                ) => {
                    d2.x += d.x;
                    d2.y += d.y;
                    return;
                }
                (
                    InputEvent::TouchMoved {
                        finger_id,
                        position,
                    },
                    InputEvent::TouchMoved {
                        finger_id: finger2,
                        position: pos2,
                    },
                ) if *finger_id == *finger2 => {
                    *pos2 = *position;
                    return;
                }
                _ => (),
            }

            self.handle_pending_event(node);
        }

        self.pending = Some(event);
    }

    fn handle_pending_event(&mut self, node: Node) {
        let Some(event) = self.pending.take() else {
            return;
        };

        match event {
            InputEvent::Focused(state) => {
                self.window_has_focus = state;
                if state {
                    // Required to restart theme animations
                    self.redraw();
                } else {
                    // Window focus lost: close all popups
                    while let Some(id) = self.popups.last().map(|state| state.id) {
                        self.close_window(id);
                    }
                }
            }
            InputEvent::KeyboardInput {
                event,
                is_synthetic,
            } => self.keyboard_input(node, event, is_synthetic),
            InputEvent::ModifiersChanged(state) => self.modifiers_changed(state),
            InputEvent::Ime(event) => self.ime_event(node, event),
            InputEvent::MouseMoved { position } => self.handle_pointer_moved(node, position),
            InputEvent::MouseEntered => self.handle_pointer_entered(),
            InputEvent::MouseLeft => self.handle_pointer_left(node),
            InputEvent::MouseWheel { delta } => self.handle_mouse_wheel(node, delta),
            InputEvent::MouseButton { button, state } => {
                self.handle_mouse_input(node, state, button)
            }
            InputEvent::TouchStart {
                finger_id,
                position,
            } => self.handle_touch_start(node, finger_id, position),
            InputEvent::TouchMoved {
                finger_id,
                position,
            } => self.handle_touch_moved(node, finger_id, position),
            InputEvent::TouchEnd {
                finger_id,
                position,
            } => self.handle_touch_end(node, finger_id, position),

            InputEvent::DataTransferReceived { id, serial, value } => {
                let Some(target) = self.data_transfers.remove(&serial) else {
                    log::warn!("received data transfer with unknown serial");
                    return;
                };

                let event = Event::DataTransfer { id, value };
                self.send_event(node, target, event);
            }
        }
    }
}
