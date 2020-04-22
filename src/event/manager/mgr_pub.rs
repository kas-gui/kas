// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use log::{debug, trace};
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::geom::Coord;
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{ThemeAction, ThemeApi, TkAction, WidgetId, WindowId};

impl<'a> std::ops::AddAssign<TkAction> for Manager<'a> {
    #[inline]
    fn add_assign(&mut self, action: TkAction) {
        self.send_action(action);
    }
}

/// Public API (around event manager state)
impl ManagerState {
    /// Get whether this widget has a grab on character input
    #[inline]
    pub fn char_focus(&self, w_id: WidgetId) -> bool {
        self.char_focus == Some(w_id)
    }

    /// Get whether this widget has keyboard focus
    #[inline]
    pub fn nav_focus(&self, w_id: WidgetId) -> bool {
        self.nav_focus == Some(w_id)
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_hovered(&self, w_id: WidgetId) -> bool {
        self.mouse_grab.is_none() && self.hover == Some(w_id)
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: WidgetId) -> bool {
        for (_, id) in &self.key_depress {
            if *id == w_id {
                return true;
            }
        }
        if self.mouse_grab.as_ref().and_then(|grab| grab.depress) == Some(w_id) {
            return true;
        }
        for touch in &self.touch_grab {
            if touch.depress == Some(w_id) {
                return true;
            }
        }
        false
    }
}

/// Public API (around toolkit functionality)
impl<'a> Manager<'a> {
    /// Schedule an update
    ///
    /// Widgets requiring animation should schedule an update; as a result,
    /// [`Event::TimerUpdate`] will be sent, roughly at time `now + duration`.
    ///
    /// Timings may be a few ms out, but should be sufficient for e.g. updating
    /// a clock each second. Very short positive durations (e.g. 1ns) may be
    /// used to schedule an update on the next frame. Frames should in any case
    /// be limited by vsync, avoiding excessive frame rates.
    ///
    /// This should be called from [`WidgetConfig::configure`] or from an event
    /// handler. Note that scheduled updates are cleared if reconfigured.
    pub fn update_on_timer(&mut self, duration: Duration, w_id: WidgetId) {
        let time = Instant::now() + duration;
        'outer: loop {
            for row in &mut self.mgr.time_updates {
                if row.1 == w_id {
                    if row.0 <= time {
                        return;
                    } else {
                        row.0 = time;
                        break 'outer;
                    }
                }
            }

            self.mgr.time_updates.push((time, w_id));
            break;
        }

        self.mgr.time_updates.sort_by(|a, b| b.cmp(a)); // reverse sort
    }

    /// Subscribe to an update handle
    ///
    /// All widgets subscribed to an update handle will be sent
    /// [`Event::HandleUpdate`] when [`Manager::trigger_update`]
    /// is called with the corresponding handle.
    ///
    /// This should be called from [`WidgetConfig::configure`].
    pub fn update_on_handle(&mut self, handle: UpdateHandle, w_id: WidgetId) {
        self.mgr
            .handle_updates
            .entry(handle)
            .or_insert(Vec::new())
            .push(w_id);
    }

    /// Notify that a widget must be redrawn
    #[inline]
    pub fn redraw(&mut self, _id: WidgetId) {
        // Theoretically, notifying by WidgetId allows selective redrawing
        // (damage events). This is not yet implemented.
        self.send_action(TkAction::Redraw);
    }

    /// Notify that a [`TkAction`] action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    /// Get the current [`TkAction`], replacing with `None`
    ///
    /// The caller is responsible for ensuring the action is handled correctly;
    /// generally this means matching only actions which can be handled locally
    /// and downgrading the action, adding the result back to the [`Manager`].
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    /// Add an overlay (pop-up)
    ///
    /// A pop-up is a box used for things like tool-tips and menus which is
    /// drawn on top of other content and has focus for input.
    ///
    /// Depending on the host environment, the pop-up may be a special type of
    /// window without borders and with precise placement, or may be a layer
    /// drawn in an existing window.
    ///
    /// The pop-up should be placed *next to* the specified `rect`, in the given
    /// `direction`.
    #[inline]
    pub fn add_popup(&mut self, popup: kas::Popup) -> WindowId {
        let id = self.tkw.add_popup(popup.clone());
        self.mgr.popups.push((id, popup));
        self.mgr.nav_focus = None;
        self.mgr.nav_stack.clear();
        id
    }

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added via event
    /// processing, albeit without error handling.
    #[inline]
    pub fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId {
        self.tkw.add_window(widget)
    }

    /// Close a window
    #[inline]
    pub fn close_window(&mut self, id: WindowId) {
        if let Some((index, parent)) = self.mgr.popups.iter().enumerate().find_map(|(i, p)| {
            if p.0 == id {
                Some((i, p.1.parent))
            } else {
                None
            }
        }) {
            self.mgr.popups.remove(index);
            self.mgr.popup_removed.push((parent, id));
            self.mgr.nav_focus = None;
            self.mgr.nav_stack.clear();
        }
        self.mgr.send_action(TkAction::RegionMoved);
        self.tkw.close_window(id);
    }

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateHandle`], across all
    /// windows, will receive an update.
    #[inline]
    pub fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.tkw.trigger_update(handle, payload);
    }

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    #[inline]
    pub fn get_clipboard(&mut self) -> Option<crate::CowString> {
        self.tkw.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard<'c>(&mut self, content: crate::CowStringL<'c>) {
        self.tkw.set_clipboard(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnMut(&mut dyn ThemeApi) -> ThemeAction>(&mut self, mut f: F) {
        self.tkw.adjust_theme(&mut f);
    }
}

/// Public API (around event manager state)
impl<'a> Manager<'a> {
    /// Attempts to set a fallback to receive [`Event::NavKey`]
    ///
    /// In case a navigation key is pressed (e.g. Left arrow) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::NavKey`]. (This does not include
    /// [`Event::Activate`].)
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow [`ScrollRegion`] to respond to
    /// navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: WidgetId) {
        if self.mgr.nav_fallback.is_none() {
            debug!("Manager: nav_fallback = {}", id);
            self.mgr.nav_fallback = Some(id);
        }
    }

    ///
    /// Adds an accelerator key for a widget
    ///
    /// If this key is pressed when the window has focus and no widget has a
    /// key-grab, the given widget will receive an [`Event::Activate`] event.
    ///
    /// This should be set from [`WidgetConfig::configure`].
    #[inline]
    pub fn add_accel_key(&mut self, key: VirtualKeyCode, id: WidgetId) {
        if !self.read_only {
            self.mgr.accel_keys.insert(key, id);
        }
    }

    /// Request character-input focus
    ///
    /// If successful, [`Event::ReceivedCharacter`] events are sent to this
    /// widget when character data is received.
    ///
    /// Currently, this method always succeeds.
    pub fn request_char_focus(&mut self, id: WidgetId) {
        if !self.read_only {
            self.set_char_focus(Some(id));
        }
    }

    /// Request a grab on the given input `source`
    ///
    /// If successful, corresponding mouse/touch events will be forwarded to
    /// this widget. The grab terminates automatically.
    ///
    /// Each grab can optionally visually depress one widget, and initially
    /// depresses the widget owning the grab (the `id` passed here). Call
    /// [`Manager::set_grab_depress`] to update the grab's depress target.
    /// This is cleared automatically when the grab ends.
    ///
    /// Behaviour depends on the `mode`:
    ///
    /// -   [`GrabMode::Grab`]: simple / low-level interpretation of input
    ///     which delivers [`Event::PressMove`] and [`Event::PressEnd`] events.
    ///     Multiple event sources may be grabbed simultaneously.
    /// -   All other [`GrabMode`] values: generates [`Event::Pan`] events.
    ///     Requesting additional grabs on the same widget from the same source
    ///     (i.e. multiple touches) allows generation of rotation and scale
    ///     factors (depending on the [`GrabMode`]).
    ///     Any previously existing `Pan` grabs by this widgets are replaced.
    ///
    /// Since these events are *requested*, the widget should consume them even
    /// if not required (e.g. [`Event::PressMove`], although in practice this
    /// only affects parents intercepting [`Response::Unhandled`] events.
    ///
    /// This method normally succeeds, but fails when
    /// multiple widgets attempt a grab the same press source simultaneously
    /// (only the first grab is successful).
    ///
    /// This method automatically cancels any active char grab
    /// and updates keyboard navigation focus.
    pub fn request_grab(
        &mut self,
        id: WidgetId,
        source: PressSource,
        coord: Coord,
        mode: GrabMode,
        cursor: Option<CursorIcon>,
    ) -> bool {
        if self.read_only {
            return false;
        }

        let start_id = id;
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button) => {
                if self.mgr.mouse_grab.is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.mgr.set_pan_on(id, mode, false, coord);
                }
                trace!("Manager: start mouse grab by {}", start_id);
                self.mgr.mouse_grab = Some(MouseGrab {
                    start_id,
                    depress: Some(id),
                    button,
                    mode,
                    pan_grab,
                });
                if let Some(icon) = cursor {
                    self.tkw.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if self.get_touch(touch_id).is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.mgr.set_pan_on(id, mode, true, coord);
                }
                trace!("Manager: start touch grab by {}", start_id);
                self.mgr.touch_grab.push(TouchGrab {
                    touch_id,
                    start_id,
                    depress: Some(id),
                    cur_id: Some(id),
                    coord,
                    mode,
                    pan_grab,
                });
            }
        }

        self.set_char_focus(None);
        self.redraw(start_id);
        true
    }

    /// Set a grab's depress target
    ///
    /// When a grab on mouse or touch input is in effect
    /// ([`Manager::request_grab`]), the widget owning the grab may set itself
    /// or any other widget as *depressed* ("pushed down"). Each grab depresses
    /// at most one widget, thus setting a new depress target clears any
    /// existing target. Initially a grab depresses its owner.
    ///
    /// This effect is purely visual. A widget is depressed when one or more
    /// grabs targets the widget to depress, or when a keyboard binding is used
    /// to activate a widget (for the duration of the key-press).
    pub fn set_grab_depress(&mut self, source: PressSource, target: Option<WidgetId>) {
        match source {
            PressSource::Mouse(_) => {
                if let Some(grab) = self.mgr.mouse_grab.as_mut() {
                    grab.depress = target;
                }
            }
            PressSource::Touch(id) => {
                for touch in &mut self.mgr.touch_grab {
                    if touch.touch_id == id {
                        touch.depress = target;
                        break;
                    }
                }
            }
        }
        self.mgr.send_action(TkAction::Redraw);
    }

    /// Set the keyboard navigation focus directly
    pub fn set_nav_focus(&mut self, id: Option<WidgetId>) {
        self.mgr.nav_focus = id;
        self.mgr.nav_stack.clear();
        self.mgr.send_action(TkAction::Redraw);
    }
}
