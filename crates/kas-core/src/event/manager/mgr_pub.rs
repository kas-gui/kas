// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use log::{debug, trace};
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::draw::DrawShared;
use crate::geom::{Coord, Offset, Vec2};
use crate::layout::SetRectMgr;
use crate::theme::{SizeMgr, ThemeControl};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{CoreData, TkAction, WidgetExt, WidgetId, WindowId};

impl<'a> std::ops::BitOrAssign<TkAction> for EventMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.send_action(action);
    }
}

/// Public API (around event manager state)
impl EventState {
    /// True when accelerator key labels should be shown
    ///
    /// (True when Alt is held and no widget has character focus.)
    ///
    /// This is a fast check.
    #[inline]
    pub fn show_accel_labels(&self) -> bool {
        self.modifiers.alt() && !self.char_focus
    }

    /// Get whether this widget has `(char_focus, sel_focus)`
    ///
    /// -   `char_focus`: implies this widget receives keyboard input
    /// -   `sel_focus`: implies this widget is allowed to select things
    ///
    /// Note that `char_focus` implies `sel_focus`.
    #[inline]
    pub fn has_char_focus(&self, w_id: &WidgetId) -> (bool, bool) {
        let sel_focus = *w_id == self.sel_focus;
        (sel_focus && self.char_focus, sel_focus)
    }

    /// Get whether this widget has keyboard navigation focus
    #[inline]
    pub fn nav_focus(&self, w_id: &WidgetId) -> bool {
        *w_id == self.nav_focus
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_hovered(&self, w_id: &WidgetId) -> bool {
        self.mouse_grab.is_none() && *w_id == self.hover
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: &WidgetId) -> bool {
        for (_, id) in &self.key_depress {
            if *id == w_id {
                return true;
            }
        }
        if self
            .mouse_grab
            .as_ref()
            .map(|grab| *w_id == grab.depress)
            .unwrap_or(false)
        {
            return true;
        }
        for grab in self.touch_grab.iter() {
            if *w_id == grab.depress {
                return true;
            }
        }
        false
    }

    /// Construct [`InputState`]
    ///
    /// The `disabled` flag is inherited from parents. [`InputState::disabled`]
    /// will be true if either `disabled` or `self.is_disabled()` are true.
    ///
    /// The error state defaults to `false` since most widgets don't support
    /// this.
    ///
    /// Note: most state changes should automatically cause a redraw, but change
    /// in `hover` status will not (since this happens frequently and many
    /// widgets are unaffected), unless [`WidgetConfig::hover_highlight`]
    /// returns true.
    pub fn draw_state(&self, core: &CoreData, disabled: bool) -> InputState {
        let (char_focus, sel_focus) = self.has_char_focus(&core.id);
        let mut state = InputState::empty();
        if core.disabled || disabled {
            state |= InputState::DISABLED;
        }
        if self.is_hovered(&core.id) {
            state |= InputState::HOVER;
        }
        if self.is_depressed(&core.id) {
            state |= InputState::DEPRESS;
        }
        if self.nav_focus(&core.id) {
            state |= InputState::NAV_FOCUS;
        }
        if char_focus {
            state |= InputState::CHAR_FOCUS;
        }
        if sel_focus {
            state |= InputState::SEL_FOCUS;
        }
        state
    }
}

/// Public API (around toolkit and shell functionality)
impl<'a> EventMgr<'a> {
    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.state.modifiers
    }

    /// Access event-handling configuration
    #[inline]
    pub fn config(&self) -> &WindowConfig {
        &self.state.config
    }

    /// Is mouse panning enabled?
    #[inline]
    pub fn config_enable_mouse_pan(&self) -> bool {
        self.state
            .config
            .mouse_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Is mouse text panning enabled?
    #[inline]
    pub fn config_enable_mouse_text_pan(&self) -> bool {
        self.state
            .config
            .mouse_text_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Test pan threshold against config, adjusted for scale factor
    ///
    /// Returns true when `dist` is large enough to switch to pan mode.
    #[inline]
    pub fn config_test_pan_thresh(&self, dist: Offset) -> bool {
        let thresh = self.state.config.pan_dist_thresh();
        Vec2::from(dist).sum_square() >= thresh * thresh
    }

    /// Access the screen's scale factor
    #[inline]
    pub fn scale_factor(&self) -> f32 {
        self.state.scale_factor
    }

    /// Schedule an update
    ///
    /// Widgets requiring animation should schedule an update; as a result,
    /// the widget will receive [`Event::TimerUpdate`] (with this `payload`)
    /// at approximately `time = now + delay`.
    ///
    /// Timings may be a few ms out, but should be sufficient for e.g. updating
    /// a clock each second. Very short positive durations (e.g. 1ns) may be
    /// used to schedule an update on the next frame. Frames should in any case
    /// be limited by vsync, avoiding excessive frame rates.
    ///
    /// If multiple updates with the same `w_id` and `payload` are requested,
    /// these are merged (using the earliest time). Updates with differing
    /// `w_id` or `payload` are not merged (since presumably they have different
    /// purposes).
    ///
    /// This may be called from [`WidgetConfig::configure`] or from an event
    /// handler. Note that previously-scheduled updates are cleared when
    /// widgets are reconfigured.
    pub fn update_on_timer(&mut self, delay: Duration, w_id: WidgetId, payload: u64) {
        trace!(
            "EventMgr::update_on_timer: queing update for {} at now+{}ms",
            w_id,
            delay.as_millis()
        );
        let time = Instant::now() + delay;
        'outer: loop {
            for row in &mut self.state.time_updates {
                if row.1 == w_id && row.2 == payload {
                    if row.0 <= time {
                        return;
                    } else {
                        row.0 = time;
                        break 'outer;
                    }
                }
            }

            self.state.time_updates.push((time, w_id, payload));
            break;
        }

        self.state.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    /// Subscribe to an update handle
    ///
    /// All widgets subscribed to an update handle will be sent
    /// [`Event::HandleUpdate`] when [`EventMgr::trigger_update`]
    /// is called with the corresponding handle.
    ///
    /// This should be called from [`WidgetConfig::configure`].
    pub fn update_on_handle(&mut self, handle: UpdateHandle, w_id: WidgetId) {
        trace!(
            "EventMgr::update_on_handle: update {} on handle {:?}",
            w_id,
            handle
        );
        self.state
            .handle_updates
            .entry(handle)
            .or_insert_with(Default::default)
            .insert(w_id);
    }

    /// Notify that a widget must be redrawn
    ///
    /// Currently the entire window is redrawn on any redraw request and the
    /// [`WidgetId`] is ignored. In the future partial redraws may be used.
    #[inline]
    pub fn redraw(&mut self, _id: WidgetId) {
        // Theoretically, notifying by WidgetId allows selective redrawing
        // (damage events). This is not yet implemented.
        self.send_action(TkAction::REDRAW);
    }

    /// Notify that a [`TkAction`] action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Calling `mgr.send_action(action)` is equivalent to `*mgr |= action`.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action |= action;
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
    /// The parent of a popup automatically receives mouse-motion events
    /// ([`Event::PressMove`]) which may be used to navigate menus.
    ///
    /// A pop-up may be closed by calling [`EventMgr::close_window`] with
    /// the [`WindowId`] returned by this method.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    #[inline]
    pub fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId> {
        trace!("Manager::add_popup({:?})", popup);
        let new_id = &popup.id;
        while let Some((_, popup, _)) = self.state.popups.last() {
            if popup.parent.is_ancestor_of(new_id) {
                break;
            }
            let (wid, popup, _old_nav_focus) = self.state.popups.pop().unwrap();
            self.shell.close_window(wid);
            self.state.popup_removed.push((popup.parent, wid));
            // Don't restore old nav focus: assume new focus will be set by new popup
        }

        let opt_id = self.shell.add_popup(popup.clone());
        if let Some(id) = opt_id {
            self.state
                .popups
                .push((id, popup, self.state.nav_focus.clone()));
        }
        self.clear_nav_focus();
        opt_id
    }

    /// Add a window
    ///
    /// Typically an application adds at least one window before the event-loop
    /// starts (see `kas_wgpu::Toolkit::add`), however this method is not
    /// available to a running UI. Instead, this method may be used.
    ///
    /// Caveat: if an error occurs opening the new window it will not be
    /// reported (except via log messages).
    #[inline]
    pub fn add_window(&mut self, widget: Box<dyn crate::Window>) -> WindowId {
        self.shell.add_window(widget)
    }

    /// Close a window or pop-up
    ///
    /// In the case of a pop-up, all pop-ups created after this will also be
    /// removed (on the assumption they are a descendant of the first popup).
    ///
    /// If `restore_focus` then navigation focus will return to whichever widget
    /// had focus before the popup was open. (Usually this is true excepting
    /// where focus has already been changed.)
    #[inline]
    pub fn close_window(&mut self, id: WindowId, restore_focus: bool) {
        if let Some(index) =
            self.state.popups.iter().enumerate().find_map(
                |(i, p)| {
                    if p.0 == id {
                        Some(i)
                    } else {
                        None
                    }
                },
            )
        {
            let mut old_nav_focus = None;
            while self.state.popups.len() > index {
                let (wid, popup, onf) = self.state.popups.pop().unwrap();
                self.state.popup_removed.push((popup.parent, wid));
                self.shell.close_window(wid);
                old_nav_focus = onf;
            }

            if !restore_focus {
                old_nav_focus = None
            }
            if let Some(id) = old_nav_focus {
                self.set_nav_focus(id, true);
            }
            // TODO: if popup.id is an ancestor of self.nav_focus then clear
            // focus if not setting (currently we cannot test this)
        }

        self.shell.close_window(id);
    }

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateHandle`], across all
    /// windows, will receive an update.
    #[inline]
    pub fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        debug!("trigger_update: handle={:?}, payload={}", handle, payload);
        self.shell.trigger_update(handle, payload);
    }

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    #[inline]
    pub fn get_clipboard(&mut self) -> Option<String> {
        self.shell.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard(&mut self, content: String) {
        self.shell.set_clipboard(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnMut(&mut dyn ThemeControl) -> TkAction>(&mut self, mut f: F) {
        self.shell.adjust_theme(&mut f);
    }

    /// Access a [`SizeMgr`]
    pub fn size_mgr<F: FnMut(SizeMgr) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(&mut |size_handle, _| {
            result = Some(f(SizeMgr::new(size_handle)));
        });
        result.expect("ShellWindow::size_handle impl failed to call function argument")
    }

    /// Access a [`SetRectMgr`]
    pub fn set_rect_mgr<F: FnMut(&mut SetRectMgr) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell
            .size_and_draw_shared(&mut |size_handle, draw_shared| {
                let mut mgr = SetRectMgr::new(size_handle, draw_shared);
                result = Some(f(&mut mgr));
                self.action |= mgr.take_action();
            });
        result.expect("ShellWindow::size_handle impl failed to call function argument")
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared<F: FnMut(&mut dyn DrawShared) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(&mut |_, draw_shared| {
            result = Some(f(draw_shared));
        });
        result.expect("ShellWindow::draw_shared impl failed to call function argument")
    }
}

/// Public API (around event manager state)
impl<'a> EventMgr<'a> {
    /// Attempts to set a fallback to receive [`Event::Command`]
    ///
    /// In case a navigation key is pressed (see [`Command`]) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::Command`]. (This does not include
    /// [`Event::Activate`].)
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow scroll-region widgets to
    /// respond to navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: WidgetId) {
        if self.state.nav_fallback.is_none() {
            debug!("EventMgr: nav_fallback = {}", id);
            self.state.nav_fallback = Some(id);
        }
    }

    fn accel_layer_for_id(&mut self, id: &WidgetId) -> Option<&mut AccelLayer> {
        let root = &WidgetId::ROOT;
        for (k, v) in self.state.accel_layers.range_mut(root..=id).rev() {
            if k.is_ancestor_of(id) {
                return Some(v);
            };
        }
        debug_assert!(false, "expected ROOT accel layer");
        None
    }

    /// Add a new accelerator key layer
    ///
    /// This method constructs a new "layer" for accelerator keys: any keys
    /// added via [`EventMgr::add_accel_keys`] to a widget which is a descentant
    /// of (or equal to) `id` will only be active when that layer is active.
    ///
    /// This method should only be called by parents of a pop-up: layers over
    /// the base layer are *only* activated by an open pop-up.
    ///
    /// If `alt_bypass` is true, then this layer's accelerator keys will be
    /// active even without Alt pressed (but only highlighted with Alt pressed).
    pub fn new_accel_layer(&mut self, id: WidgetId, alt_bypass: bool) {
        self.state
            .accel_layers
            .insert(id, (alt_bypass, HashMap::new()));
    }

    /// Enable `alt_bypass` for layer
    ///
    /// This may be called by a child widget during configure to enable or
    /// disable alt-bypass for the accel-key layer containing its accel keys.
    /// This allows accelerator keys to be used as shortcuts without the Alt
    /// key held. See also [`EventMgr::new_accel_layer`].
    pub fn enable_alt_bypass(&mut self, id: &WidgetId, alt_bypass: bool) {
        if let Some(layer) = self.accel_layer_for_id(id) {
            layer.0 = alt_bypass;
        }
    }

    /// Adds an accelerator key for a widget
    ///
    /// An *accelerator key* is a shortcut key able to directly open menus,
    /// activate buttons, etc. A user triggers the key by pressing `Alt+Key`,
    /// or (if `alt_bypass` is enabled) by simply pressing the key.
    /// The widget with this `id` then receives [`Event::Activate`].
    ///
    /// Note that accelerator keys may be automatically derived from labels:
    /// see [`crate::text::AccelString`].
    ///
    /// Accelerator keys are added to the layer with the longest path which is
    /// an ancestor of `id`. This usually means that if the widget is part of a
    /// pop-up, the key is only active when that pop-up is open.
    /// See [`EventMgr::new_accel_layer`].
    ///
    /// This should only be called from [`WidgetConfig::configure`].
    // TODO(type safety): consider only implementing on ConfigureManager
    #[inline]
    pub fn add_accel_keys(&mut self, id: &WidgetId, keys: &[VirtualKeyCode]) {
        if let Some(layer) = self.accel_layer_for_id(id) {
            for key in keys {
                layer.1.insert(*key, id.clone());
            }
        }
    }

    /// Request character-input focus
    ///
    /// Returns true on success or when the widget already had char focus.
    ///
    /// Character data is sent to the widget with char focus via
    /// [`Event::ReceivedCharacter`] and [`Event::Command`].
    ///
    /// Char focus implies sel focus (see [`Self::request_sel_focus`]) and
    /// navigation focus.
    ///
    /// When char focus is lost, [`Event::LostCharFocus`] is sent.
    #[inline]
    pub fn request_char_focus(&mut self, id: WidgetId) -> bool {
        self.set_sel_focus(id, true);
        true
    }

    /// Request selection focus
    ///
    /// Returns true on success or when the widget already had sel focus.
    ///
    /// To prevent multiple simultaneous selections (e.g. of text) in the UI,
    /// only widgets with "selection focus" are allowed to select things.
    /// Selection focus is implied by character focus. [`Event::LostSelFocus`]
    /// is sent when selection focus is lost; in this case any existing
    /// selection should be cleared.
    ///
    /// Selection focus implies navigation focus.
    ///
    /// When char focus is lost, [`Event::LostSelFocus`] is sent.
    #[inline]
    pub fn request_sel_focus(&mut self, id: WidgetId) -> bool {
        self.set_sel_focus(id, false);
        true
    }

    /// Grab "press" events for `source` (a mouse or finger)
    ///
    /// When a "press" source is "grabbed", events for this source will be sent
    /// to the grabbing widget. Notes:
    ///
    /// -   For mouse sources, a click-press event from another button will
    ///     cancel this grab; [`Event::PressEnd`] will be sent (with mode
    ///     [`GrabMode::Grab`]), then [`Event::PressStart`] will be sent to the
    ///     widget under the mouse like normal.
    /// -   For touch-screen sources, events are delivered until the finger is
    ///     removed or the touch is cancelled (e.g. by dragging off-screen).
    /// -   [`Self::grab_press_unique`] is a variant of this method which
    ///     cancels grabs of other sources by the same widget.
    ///
    /// Each grab can optionally visually depress one widget, and initially
    /// depresses the widget owning the grab (the `id` passed here). Call
    /// [`EventMgr::set_grab_depress`] to update the grab's depress target.
    /// This is cleared automatically when the grab ends.
    ///
    /// The events sent depends on the `mode`:
    ///
    /// -   [`GrabMode::Grab`]: simple / low-level interpretation of input
    ///     which delivers [`Event::PressMove`] and [`Event::PressEnd`] events.
    /// -   All other [`GrabMode`] values: generates [`Event::Pan`] events.
    ///     Requesting additional grabs on the same widget from the same source
    ///     (i.e. multiple touches) allows generation of rotation and scale
    ///     factors (depending on the [`GrabMode`]).
    ///     Any previously existing `Pan` grabs by this widgets are replaced.
    ///
    /// Since these events are *requested*, the widget should consume them even
    /// if not required, although in practice this
    /// only affects parents intercepting [`Response::Unused`] events.
    pub fn grab_press(
        &mut self,
        id: WidgetId,
        source: PressSource,
        coord: Coord,
        mode: GrabMode,
        cursor: Option<CursorIcon>,
    ) {
        let start_id = id.clone();
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button, repetitions) => {
                if self.remove_mouse_grab().is_some() {
                    #[cfg(debug_assertions)]
                    log::error!("grab_press: existing mouse grab!");
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.state.set_pan_on(id.clone(), mode, false, coord);
                }
                trace!("EventMgr: start mouse grab by {}", start_id);
                self.state.mouse_grab = Some(MouseGrab {
                    button,
                    repetitions,
                    start_id: start_id.clone(),
                    cur_id: Some(start_id.clone()),
                    depress: Some(id),
                    mode,
                    pan_grab,
                    coord,
                    delta: Offset::ZERO,
                });
                if let Some(icon) = cursor {
                    self.shell.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if self.remove_touch(touch_id).is_some() {
                    #[cfg(debug_assertions)]
                    log::error!("grab_press: existing touch grab!");
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.state.set_pan_on(id.clone(), mode, true, coord);
                }
                trace!("EventMgr: start touch grab by {}", start_id);
                self.state.touch_grab.push(TouchGrab {
                    id: touch_id,
                    start_id: start_id.clone(),
                    depress: Some(id.clone()),
                    cur_id: Some(id),
                    last_move: coord,
                    coord,
                    mode,
                    pan_grab,
                });
            }
        }

        self.redraw(start_id);
    }

    /// A variant of [`Self::grab_press`], where a unique grab is desired
    ///
    /// This removes any existing press-grabs by widget `id`, then calls
    /// [`Self::grab_press`] with `mode = GrabMode::Grab` to create a new grab.
    /// Previous grabs are discarded without delivering [`Event::PressEnd`].
    pub fn grab_press_unique(
        &mut self,
        id: WidgetId,
        source: PressSource,
        coord: Coord,
        cursor: Option<CursorIcon>,
    ) {
        if id == self.state.mouse_grab.as_ref().map(|grab| &grab.start_id) {
            self.remove_mouse_grab();
        }
        self.state.touch_grab.retain(|grab| id != grab.start_id);

        self.grab_press(id, source, coord, GrabMode::Grab, cursor);
    }

    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`EventMgr::grab_press`]). The cursor will be reset when the mouse-grab
    /// ends.
    pub fn update_grab_cursor(&mut self, id: WidgetId, icon: CursorIcon) {
        if let Some(ref grab) = self.state.mouse_grab {
            if grab.start_id == id {
                self.shell.set_cursor_icon(icon);
            }
        }
    }

    /// Set a grab's depress target
    ///
    /// When a grab on mouse or touch input is in effect
    /// ([`EventMgr::grab_press`]), the widget owning the grab may set itself
    /// or any other widget as *depressed* ("pushed down"). Each grab depresses
    /// at most one widget, thus setting a new depress target clears any
    /// existing target. Initially a grab depresses its owner.
    ///
    /// This effect is purely visual. A widget is depressed when one or more
    /// grabs targets the widget to depress, or when a keyboard binding is used
    /// to activate a widget (for the duration of the key-press).
    ///
    /// Queues a redraw and returns `true` if the depress target changes,
    /// otherwise returns `false`.
    pub fn set_grab_depress(&mut self, source: PressSource, target: Option<WidgetId>) -> bool {
        let mut redraw = false;
        match source {
            PressSource::Mouse(_, _) => {
                if let Some(grab) = self.state.mouse_grab.as_mut() {
                    redraw = grab.depress != target;
                    grab.depress = target.clone();
                }
            }
            PressSource::Touch(id) => {
                if let Some(grab) = self.get_touch(id) {
                    redraw = grab.depress != target;
                    grab.depress = target.clone();
                }
            }
        }
        if redraw {
            trace!("EventMgr: set_grab_depress on target={:?}", target);
            self.state.send_action(TkAction::REDRAW);
        }
        redraw
    }

    /// Returns true if `id` or any descendant has a mouse or touch grab
    pub fn any_pin_on(&self, id: &WidgetId) -> bool {
        if self
            .state
            .mouse_grab
            .as_ref()
            .map(|grab| grab.start_id == id)
            .unwrap_or(false)
        {
            return true;
        }
        if self.state.touch_grab.iter().any(|grab| grab.start_id == id) {
            return true;
        }
        false
    }

    /// Get the current keyboard navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    #[inline]
    pub fn nav_focus(&self) -> Option<&WidgetId> {
        self.state.nav_focus.as_ref()
    }

    /// Clear keyboard navigation focus
    pub fn clear_nav_focus(&mut self) {
        if let Some(id) = self.state.nav_focus.clone() {
            self.redraw(id);
        }
        self.state.nav_focus = None;
        self.clear_char_focus();
        trace!("EventMgr: nav_focus = None");
    }

    /// Set the keyboard navigation focus directly
    ///
    /// Normally, [`WidgetConfig::key_nav`] will be true for the specified
    /// widget, but this is not required, e.g. a `ScrollLabel` can receive focus
    /// on text selection with the mouse. (Currently such widgets will receive
    /// events like any other with nav focus, but this may change.)
    ///
    /// The target widget, if not already having navigation focus, will receive
    /// [`Event::NavFocus`] with `key_focus` as the payload. This boolean should
    /// be true if focussing in response to keyboard input, false if reacting to
    /// mouse or touch input.
    pub fn set_nav_focus(&mut self, id: WidgetId, key_focus: bool) {
        if id != self.state.nav_focus {
            self.redraw(id.clone());
            if id != self.state.sel_focus {
                self.clear_char_focus();
            }
            self.state.nav_focus = Some(id.clone());
            trace!("EventMgr: nav_focus = Some({})", id);
            self.state.pending.push(Pending::SetNavFocus(id, key_focus));
        }
    }

    /// Advance the keyboard navigation focus
    ///
    /// If some widget currently has nav focus, this will give focus to the next
    /// (or previous) widget under `widget` where [`WidgetConfig::key_nav`]
    /// returns true; otherwise this will give focus to the first (or last)
    /// such widget.
    ///
    /// Returns true on success, false if there are no navigable widgets or
    /// some error occurred.
    ///
    /// The target widget will receive [`Event::NavFocus`] with `key_focus` as
    /// the payload. This boolean should be true if focussing in response to
    /// keyboard input, false if reacting to mouse or touch input.
    pub fn next_nav_focus(
        &mut self,
        mut widget: &mut dyn WidgetConfig,
        reverse: bool,
        key_focus: bool,
    ) -> bool {
        if let Some(id) = self.state.popups.last().map(|(_, p, _)| p.id.clone()) {
            if let Some(w) = widget.find_widget_mut(&id) {
                widget = w;
            } else {
                // This is a corner-case. Do nothing.
                return false;
            }
        }

        // We redraw in all cases. Since this is not part of widget event
        // processing, we can push directly to self.state.action.
        self.state.send_action(TkAction::REDRAW);

        fn nav(
            mgr: &mut SetRectMgr,
            widget: &mut dyn WidgetConfig,
            focus: Option<&WidgetId>,
            rev: bool,
        ) -> Option<WidgetId> {
            if widget.is_disabled() {
                return None;
            }

            let mut child = focus.and_then(|id| widget.find_child_index(id));

            if !rev {
                if let Some(index) = child {
                    if let Some(id) = widget
                        .get_child_mut(index)
                        .and_then(|w| nav(mgr, w, focus, rev))
                    {
                        return Some(id);
                    }
                } else if !widget.eq_id(focus) && widget.key_nav() {
                    return Some(widget.id());
                }

                loop {
                    if let Some(index) = widget.spatial_nav(mgr, rev, child) {
                        if let Some(id) = widget
                            .get_child_mut(index)
                            .and_then(|w| nav(mgr, w, focus, rev))
                        {
                            return Some(id);
                        }
                        child = Some(index);
                    } else {
                        return None;
                    }
                }
            } else {
                if let Some(index) = child {
                    if let Some(id) = widget
                        .get_child_mut(index)
                        .and_then(|w| nav(mgr, w, focus, rev))
                    {
                        return Some(id);
                    }
                }

                loop {
                    if let Some(index) = widget.spatial_nav(mgr, rev, child) {
                        if let Some(id) = widget
                            .get_child_mut(index)
                            .and_then(|w| nav(mgr, w, focus, rev))
                        {
                            return Some(id);
                        }
                        child = Some(index);
                    } else {
                        return if !widget.eq_id(focus) && widget.key_nav() {
                            Some(widget.id())
                        } else {
                            None
                        };
                    }
                }
            }
        }

        // Whether to restart from the beginning on failure
        let restart = self.state.nav_focus.is_some();

        let focus = self.state.nav_focus.clone();
        let mut opt_id = None;
        self.set_rect_mgr(|mgr| {
            opt_id = nav(mgr, widget, focus.as_ref(), reverse);
            if restart && opt_id.is_none() {
                opt_id = nav(mgr, widget, None, reverse);
            }
        });

        trace!("EventMgr: nav_focus = {:?}", opt_id);
        self.state.nav_focus = opt_id.clone();

        if let Some(id) = opt_id {
            if id != self.state.sel_focus {
                self.clear_char_focus();
            }
            self.state.pending.push(Pending::SetNavFocus(id, key_focus));
            true
        } else {
            // Most likely an error occurred
            self.clear_char_focus();
            self.state.nav_focus = None;
            false
        }
    }
}
