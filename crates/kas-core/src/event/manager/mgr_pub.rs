// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use log::{debug, error, trace};
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::draw::{DrawShared, SizeHandle, ThemeApi};
use crate::geom::{Coord, Offset, Vec2};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{TkAction, WidgetId, WindowId};

impl<'a> std::ops::BitOrAssign<TkAction> for Manager<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.send_action(action);
    }
}

/// Public API (around event manager state)
impl ManagerState {
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
    pub fn has_char_focus(&self, w_id: WidgetId) -> (bool, bool) {
        if let Some(id) = self.sel_focus {
            if id == w_id {
                return (self.char_focus, true);
            }
        }
        (false, false)
    }

    /// Get whether this widget has keyboard navigation focus
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
        for grab in self.touch_grab.values() {
            if grab.depress == Some(w_id) {
                return true;
            }
        }
        false
    }
}

/// Public API (around toolkit and shell functionality)
impl<'a> Manager<'a> {
    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.state.modifiers
    }

    /// Access event-handling configuration
    #[inline]
    pub fn config(&self) -> impl std::ops::Deref<Target = Config> + '_ {
        self.state.config.borrow()
    }

    /// Is mouse panning enabled?
    #[inline]
    pub fn config_enable_mouse_pan(&self) -> bool {
        self.config().mouse_pan().is_enabled_with(self.modifiers())
    }

    /// Is mouse text panning enabled?
    #[inline]
    pub fn config_enable_mouse_text_pan(&self) -> bool {
        self.config()
            .mouse_text_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Test pan threshold against config, adjusted for scale factor
    ///
    /// Returns true when `dist` is large enough to switch to pan mode.
    #[inline]
    pub fn config_test_pan_thresh(&self, dist: Offset) -> bool {
        let thresh = self.config().pan_dist_thresh() * self.scale_factor();
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
            "Manager::update_on_timer: queing update for {} at now+{}ms",
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
    /// [`Event::HandleUpdate`] when [`Manager::trigger_update`]
    /// is called with the corresponding handle.
    ///
    /// This should be called from [`WidgetConfig::configure`].
    pub fn update_on_handle(&mut self, handle: UpdateHandle, w_id: WidgetId) {
        trace!(
            "Manager::update_on_handle: update {} on handle {:?}",
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
    /// A pop-up may be closed by calling [`Manager::close_window`] with
    /// the [`WindowId`] returned by this method.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    #[inline]
    pub fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId> {
        let opt_id = self.shell.add_popup(popup.clone());
        if let Some(id) = opt_id {
            self.state.new_popups.push(popup.id);
            self.state.popups.push((id, popup, self.state.nav_focus));
            self.clear_nav_focus();
        }
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
    pub fn adjust_theme<F: FnMut(&mut dyn ThemeApi) -> TkAction>(&mut self, mut f: F) {
        self.shell.adjust_theme(&mut f);
    }

    /// Access a [`SizeHandle`]
    pub fn size_handle<F: FnMut(&mut dyn SizeHandle) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_handle(&mut |size_handle| {
            result = Some(f(size_handle));
        });
        result.expect("ShellWindow::size_handle impl failed to call function argument")
    }

    /// Access a [`DrawShared`]
    ///
    /// This can be accessed through [`Self::size_handle`]; this method is merely a shortcut.
    pub fn draw_shared<F: FnMut(&mut dyn DrawShared) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.draw_shared(&mut |draw_shared| {
            result = Some(f(draw_shared));
        });
        result.expect("ShellWindow::draw_shared impl failed to call function argument")
    }
}

/// Public API (around event manager state)
impl<'a> Manager<'a> {
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
            debug!("Manager: nav_fallback = {}", id);
            self.state.nav_fallback = Some(id);
        }
    }

    /// Add a new accelerator key layer and make it current
    ///
    /// This method affects the behaviour of [`Manager::add_accel_keys`] by
    /// adding a new *layer* and making this new layer *current*.
    ///
    /// This method should only be called by parents of a pop-up: layers over
    /// the base layer are *only* activated by an open pop-up.
    ///
    /// [`Manager::pop_accel_layer`] must be called after child widgets have
    /// been configured to finish configuration of this new layer and to make
    /// the previous layer current.
    ///
    /// If `alt_bypass` is true, then this layer's accelerator keys will be
    /// active even without Alt pressed (but only highlighted with Alt pressed).
    pub fn push_accel_layer(&mut self, alt_bypass: bool) {
        self.state.accel_stack.push((alt_bypass, HashMap::new()));
    }

    /// Enable `alt_bypass` for the current layer
    ///
    /// This may be called by a child widget during configure, e.g. to enable
    /// alt-bypass for the base layer. See also [`Manager::push_accel_layer`].
    pub fn enable_alt_bypass(&mut self, alt_bypass: bool) {
        if let Some(layer) = self.state.accel_stack.last_mut() {
            layer.0 = alt_bypass;
        }
    }

    /// Finish configuration of an accelerator key layer
    ///
    /// This must be called after [`Manager::push_accel_layer`], after
    /// configuration of any children using this layer.
    ///
    /// The `id` must be that of the widget which created this layer.
    pub fn pop_accel_layer(&mut self, id: WidgetId) {
        if let Some(layer) = self.state.accel_stack.pop() {
            self.state.accel_layers.insert(id, layer);
        } else {
            debug_assert!(
                false,
                "pop_accel_layer without corresponding push_accel_layer"
            );
        }
    }

    /// Adds an accelerator key for a widget to the current layer
    ///
    /// An *accelerator key* is a shortcut key able to directly open menus,
    /// activate buttons, etc. A user triggers the key by pressing `Alt+Key`,
    /// or (if `alt_bypass` is enabled) by simply pressing the key.
    /// The widget with this `id` then receives [`Event::Activate`].
    ///
    /// Note that accelerator keys may be automatically derived from labels:
    /// see [`crate::text::AccelString`].
    ///
    /// Accelerator keys may be added to the base layer or to a new layer
    /// associated with a pop-up (see [`Manager::push_accel_layer`]).
    /// The top-most active layer gets first priority in matching input, but
    /// does not block previous layers.
    ///
    /// This should only be called from [`WidgetConfig::configure`].
    // TODO(type safety): consider only implementing on ConfigureManager
    #[inline]
    pub fn add_accel_keys(&mut self, id: WidgetId, keys: &[VirtualKeyCode]) {
        if let Some(last) = self.state.accel_stack.last_mut() {
            for key in keys {
                last.1.insert(*key, id);
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

    /// Request a grab on the given input `source`
    ///
    /// On success, this method returns true and corresponding mouse/touch
    /// events will be forwarded to widget `id`. The grab terminates
    /// automatically when the press is released.
    /// Returns false when the grab-request fails.
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
    /// if not required, although in practice this
    /// only affects parents intercepting [`Response::Unhandled`] events.
    ///
    /// This method normally succeeds, but fails when
    /// multiple widgets attempt a grab the same press source simultaneously
    /// (only the first grab is successful).
    ///
    /// This method automatically cancels any active character grab
    /// on other widgets and updates keyboard navigation focus.
    pub fn request_grab(
        &mut self,
        id: WidgetId,
        source: PressSource,
        coord: Coord,
        mode: GrabMode,
        cursor: Option<CursorIcon>,
    ) -> bool {
        let start_id = id;
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button, repetitions) => {
                if self.state.mouse_grab.is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.state.set_pan_on(id, mode, false, coord);
                }
                trace!("Manager: start mouse grab by {}", start_id);
                self.state.mouse_grab = Some(MouseGrab {
                    button,
                    repetitions,
                    start_id,
                    depress: Some(id),
                    mode,
                    pan_grab,
                });
                if let Some(icon) = cursor {
                    self.shell.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if self.get_touch(touch_id).is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.state.set_pan_on(id, mode, true, coord);
                }
                trace!("Manager: start touch grab by {}", start_id);
                self.state.touch_grab.insert(
                    touch_id,
                    TouchGrab {
                        start_id,
                        depress: Some(id),
                        cur_id: Some(id),
                        coord,
                        mode,
                        pan_grab,
                    },
                );
            }
        }

        self.redraw(start_id);
        true
    }

    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`Manager::request_grab`]). The cursor will be reset when the mouse-grab
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
    /// ([`Manager::request_grab`]), the widget owning the grab may set itself
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
                    grab.depress = target;
                }
            }
            PressSource::Touch(id) => {
                if let Some(grab) = self.state.touch_grab.get_mut(&id) {
                    redraw = grab.depress != target;
                    grab.depress = target;
                }
            }
        }
        if redraw {
            trace!("Manager: set_grab_depress on target={:?}", target);
            self.state.send_action(TkAction::REDRAW);
        }
        redraw
    }

    /// Get the current keyboard navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    pub fn nav_focus(&self) -> Option<WidgetId> {
        self.state.nav_focus
    }

    /// Clear keyboard navigation focus
    pub fn clear_nav_focus(&mut self) {
        if let Some(id) = self.state.nav_focus {
            self.redraw(id);
        }
        self.state.nav_focus = None;
        self.clear_char_focus();
        trace!("Manager: nav_focus = None");
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
        if self.state.nav_focus != Some(id) {
            self.redraw(id);
            if self.state.sel_focus != Some(id) {
                self.clear_char_focus();
            }
            self.state.nav_focus = Some(id);
            trace!("Manager: nav_focus = Some({})", id);
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
        if let Some(id) = self.state.popups.last().map(|(_, p, _)| p.id) {
            if let Some(w) = widget.find_leaf_mut(id) {
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
            mgr: &mut Manager,
            widget: &mut dyn WidgetConfig,
            focus: Option<WidgetId>,
            rev: bool,
        ) -> Option<WidgetId> {
            let last = widget.num_children().wrapping_sub(1);
            if widget.is_disabled() {
                return None;
            } else if last == usize::MAX {
                if focus != Some(widget.id()) && widget.key_nav() {
                    return Some(widget.id());
                }
                return None;
            }

            let mut child = None;
            if let Some(id) = focus {
                // Checking is_ancestor_of is just an optimisation
                if widget.is_ancestor_of(id) && id != widget.id() {
                    // TODO(opt): add WidgetChildren::find_ancestor_of method to
                    // allow optimisations for widgets with many children?
                    for index in 0..=last {
                        if widget
                            .get_child(index)
                            .map(|w| w.is_ancestor_of(id))
                            .unwrap_or(false)
                        {
                            child = Some(index);
                            break;
                        }
                    }

                    if child.is_none() {
                        error!("unable to find widget {}", id);
                        return None;
                    }
                }
            }

            if !rev {
                if let Some(index) = child {
                    if let Some(id) = widget
                        .get_child_mut(index)
                        .and_then(|w| nav(mgr, w, focus, rev))
                    {
                        return Some(id);
                    }
                } else if focus != Some(widget.id()) && widget.key_nav() {
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
                        return if focus != Some(widget.id()) && widget.key_nav() {
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

        let mut opt_id = nav(self, widget, self.state.nav_focus, reverse);
        if restart && opt_id.is_none() {
            opt_id = nav(self, widget, None, reverse);
        }

        trace!("Manager: nav_focus = {:?}", opt_id);
        self.state.nav_focus = opt_id;

        if let Some(id) = opt_id {
            if self.state.sel_focus != Some(id) {
                self.clear_char_focus();
            }
            self.state.pending.push(Pending::SetNavFocus(id, key_focus));
        } else {
            // Most likely an error occurred
            self.clear_char_focus();
            self.state.nav_focus = None;
        }
        opt_id.is_some()
    }
}
