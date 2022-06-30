// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use log::{debug, trace};
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::cast::Conv;
use crate::draw::DrawShared;
use crate::geom::{Coord, Offset, Vec2};
use crate::theme::{SizeMgr, ThemeControl};
#[allow(unused)]
use crate::Widget; // for doc-links
use crate::{TkAction, WidgetId, WindowId};

impl<'a> std::ops::BitOrAssign<TkAction> for EventMgr<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: TkAction) {
        self.send_action(action);
    }
}

/// Public API
impl EventState {
    /// True when the window has focus
    #[inline]
    pub fn window_has_focus(&self) -> bool {
        self.window_has_focus
    }

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
    pub fn has_nav_focus(&self, w_id: &WidgetId) -> bool {
        *w_id == self.nav_focus
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_hovered(&self, w_id: &WidgetId) -> bool {
        self.mouse_grab.is_none() && *w_id == self.hover
    }

    /// Check whether the given widget is visually depressed
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
        for popup in &self.popups {
            if *w_id == popup.1.parent {
                return true;
            }
        }
        false
    }

    /// Check whether a widget is disabled
    ///
    /// A widget is disabled if any ancestor is.
    #[inline]
    pub fn is_disabled(&self, w_id: &WidgetId) -> bool {
        // TODO(opt): we should be able to use binary search here
        for id in &self.disabled {
            if id.is_ancestor_of(w_id) {
                return true;
            }
        }
        false
    }

    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
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
            || source.is_primary() && self.config.mouse_pan().is_enabled_with(self.modifiers())
    }

    /// Is mouse text panning enabled?
    #[inline]
    pub fn config_enable_mouse_text_pan(&self) -> bool {
        self.config
            .mouse_text_pan()
            .is_enabled_with(self.modifiers())
    }

    /// Test pan threshold against config, adjusted for scale factor
    ///
    /// Returns true when `dist` is large enough to switch to pan mode.
    #[inline]
    pub fn config_test_pan_thresh(&self, dist: Offset) -> bool {
        let thresh = self.config.pan_dist_thresh();
        Vec2::conv(dist).sum_square() >= thresh * thresh
    }

    /// Set/unset a widget as disabled
    ///
    /// Disabled status applies to all descendants and blocks reception of
    /// events ([`Response::Unused`] is returned automatically when the
    /// recipient or any ancestor is disabled).
    pub fn set_disabled(&mut self, w_id: WidgetId, state: bool) {
        for (i, id) in self.disabled.iter().enumerate() {
            if w_id == id {
                if !state {
                    self.redraw(w_id);
                    self.disabled.remove(i);
                }
                return;
            }
        }
        if state {
            self.send_action(TkAction::REDRAW);
            self.disabled.push(w_id);
        }
    }

    /// Schedule an update
    ///
    /// Widget updates may be used for animation and timed responses. See also
    /// [`Draw::animate`](crate::draw::Draw::animate) for animation.
    ///
    /// Widget `w_id` will receive [`Event::TimerUpdate`] with this `payload` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid, except from an
    /// [`Event::TimerUpdate`] handler (where it may cause an infinite loop).
    ///
    /// If multiple updates with the same `id` and `payload` are requested,
    /// these are merged (using the earliest time if `first` is true).
    pub fn request_update(&mut self, id: WidgetId, payload: u64, delay: Duration, first: bool) {
        let time = Instant::now() + delay;
        if let Some(row) = self
            .time_updates
            .iter_mut()
            .find(|row| row.1 == id && row.2 == payload)
        {
            if (first && row.0 <= time) || (!first && row.0 >= time) {
                return;
            }

            row.0 = time;
            trace!(
                "EventMgr::request_update: update {id} at now+{}ms",
                delay.as_millis()
            );
        } else {
            self.time_updates.push((time, id, payload));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    /// Notify that a widget must be redrawn
    ///
    /// Note: currently, only full-window redraws are supported, thus this is
    /// equivalent to: `mgr.send_action(TkAction::REDRAW);`
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

    /// Attempts to set a fallback to receive [`Event::Command`]
    ///
    /// In case a navigation key is pressed (see [`Command`]) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::Command`].
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow scroll-region widgets to
    /// respond to navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: WidgetId) {
        if self.nav_fallback.is_none() {
            debug!("EventMgr: nav_fallback = {}", id);
            self.nav_fallback = Some(id);
        }
    }

    fn accel_layer_for_id(&mut self, id: &WidgetId) -> Option<&mut AccelLayer> {
        let root = &WidgetId::ROOT;
        for (k, v) in self.accel_layers.range_mut(root..=id).rev() {
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
    /// added via [`EventState::add_accel_keys`] to a widget which is a descentant
    /// of (or equal to) `id` will only be active when that layer is active.
    ///
    /// This method should only be called by parents of a pop-up: layers over
    /// the base layer are *only* activated by an open pop-up.
    ///
    /// If `alt_bypass` is true, then this layer's accelerator keys will be
    /// active even without Alt pressed (but only highlighted with Alt pressed).
    pub fn new_accel_layer(&mut self, id: WidgetId, alt_bypass: bool) {
        self.accel_layers.insert(id, (alt_bypass, HashMap::new()));
    }

    /// Enable `alt_bypass` for layer
    ///
    /// This may be called by a child widget during configure to enable or
    /// disable alt-bypass for the accel-key layer containing its accel keys.
    /// This allows accelerator keys to be used as shortcuts without the Alt
    /// key held. See also [`EventState::new_accel_layer`].
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
    /// The widget with this `id` then receives [`Command::Activate`].
    ///
    /// Note that accelerator keys may be automatically derived from labels:
    /// see [`crate::text::AccelString`].
    ///
    /// Accelerator keys are added to the layer with the longest path which is
    /// an ancestor of `id`. This usually means that if the widget is part of a
    /// pop-up, the key is only active when that pop-up is open.
    /// See [`EventState::new_accel_layer`].
    ///
    /// This should only be called from [`Widget::configure`].
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
                if let Some(grab) = self.mouse_grab.as_mut() {
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
            self.send_action(TkAction::REDRAW);
        }
        redraw
    }

    /// Returns true if `id` or any descendant has a mouse or touch grab
    pub fn any_pin_on(&self, id: &WidgetId) -> bool {
        if self
            .mouse_grab
            .as_ref()
            .map(|grab| grab.start_id == id)
            .unwrap_or(false)
        {
            return true;
        }
        if self.touch_grab.iter().any(|grab| grab.start_id == id) {
            return true;
        }
        false
    }

    /// Get the current keyboard navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    #[inline]
    pub fn nav_focus(&self) -> Option<&WidgetId> {
        self.nav_focus.as_ref()
    }

    /// Clear keyboard navigation focus
    pub fn clear_nav_focus(&mut self) {
        if let Some(id) = self.nav_focus.take() {
            self.send_action(TkAction::REDRAW);
            self.pending.push_back(Pending::LostNavFocus(id));
        }
        self.clear_char_focus();
        trace!("EventMgr: nav_focus = None");
    }

    /// Set the keyboard navigation focus directly
    ///
    /// Normally, [`Widget::key_nav`] will be true for the specified
    /// widget, but this is not required, e.g. a `ScrollLabel` can receive focus
    /// on text selection with the mouse. (Currently such widgets will receive
    /// events like any other with nav focus, but this may change.)
    ///
    /// The target widget, if not already having navigation focus, will receive
    /// [`Event::NavFocus`] with `key_focus` as the payload. This boolean should
    /// be true if focussing in response to keyboard input, false if reacting to
    /// mouse or touch input.
    pub fn set_nav_focus(&mut self, id: WidgetId, key_focus: bool) {
        if id != self.nav_focus {
            self.send_action(TkAction::REDRAW);
            if let Some(old_id) = self.nav_focus.take() {
                self.pending.push_back(Pending::LostNavFocus(old_id));
            }
            if id != self.sel_focus {
                self.clear_char_focus();
            }
            self.nav_focus = Some(id.clone());
            trace!("EventMgr: nav_focus = Some({})", id);
            self.pending.push_back(Pending::SetNavFocus(id, key_focus));
        }
    }

    /// Set the cursor icon
    ///
    /// This is normally called when handling [`Event::MouseHover`]. In other
    /// cases, calling this method may be ineffective. The cursor is
    /// automatically "unset" when the widget is no longer hovered.
    ///
    /// If a mouse grab ([`EventMgr::grab_press`]) is active, its icon takes precedence.
    pub fn set_cursor_icon(&mut self, icon: CursorIcon) {
        // Note: this is acted on by EventState::update
        self.hover_icon = icon;
    }
}

/// Public API
impl<'a> EventMgr<'a> {
    /// Send an event to a widget
    ///
    /// Messages may be left on the stack after this returns and scroll state
    /// may be adjusted.
    pub fn send(&mut self, widget: &mut dyn Widget, mut id: WidgetId, event: Event) -> Response {
        trace!("EventMgr::send to {}: {:?}", id, event);

        // TODO(opt): we should be able to use binary search here
        let mut disabled = false;
        if !event.pass_when_disabled() {
            for d in &self.disabled {
                if d.is_ancestor_of(&id) {
                    id = d.clone();
                    disabled = true;
                }
            }
            if disabled {
                trace!("EventMgr::send: target is disabled; sending instead to {id}");
            }
        }

        self.scroll = Scroll::None;
        self.send_recurse(widget, id, disabled, event)
    }

    /// Push a message to the stack
    pub fn push_msg<M: Debug + 'static>(&mut self, msg: M) {
        self.push_boxed_msg(Box::new(msg));
    }

    /// Push a pre-boxed message to the stack
    pub fn push_boxed_msg<M: Debug + 'static>(&mut self, msg: Box<M>) {
        self.messages.push(Message::new(msg));
    }

    /// True if the message stack is non-empty
    pub fn has_msg(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Try popping the last message from the stack with the given type
    pub fn try_pop_msg<M: Debug + 'static>(&mut self) -> Option<M> {
        self.try_pop_boxed_msg().map(|m| *m)
    }

    /// Try popping the last message from the stack with the given type
    pub fn try_pop_boxed_msg<M: Debug + 'static>(&mut self) -> Option<Box<M>> {
        if self.messages.last().map(|m| m.is::<M>()).unwrap_or(false) {
            self.messages.pop().unwrap().downcast::<M>().ok()
        } else {
            None
        }
    }

    /// Set a scroll action
    ///
    /// When setting [`Scroll::Rect`], use the widgets own coordinate space.
    ///
    /// Note that calling this method has no effect on the widget itself, but
    /// affects parents via their [`Widget::handle_scroll`] method.
    #[inline]
    pub fn set_scroll(&mut self, scroll: Scroll) {
        self.scroll = scroll;
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
    /// The parent automatically receives the "depressed" visual state.
    ///
    /// It is recommended to call [`EventState::set_nav_focus`] or
    /// [`EventMgr::next_nav_focus`] after this method.
    ///
    /// A pop-up may be closed by calling [`EventMgr::close_window`] with
    /// the [`WindowId`] returned by this method.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    pub fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId> {
        trace!("Manager::add_popup({:?})", popup);
        let new_id = &popup.id;
        while let Some((_, popup, _)) = self.popups.last() {
            if popup.parent.is_ancestor_of(new_id) {
                break;
            }
            let (wid, popup, _old_nav_focus) = self.popups.pop().unwrap();
            self.shell.close_window(wid);
            self.popup_removed.push((popup.parent, wid));
            // Don't restore old nav focus: assume new focus will be set by new popup
        }

        let opt_id = self.shell.add_popup(popup.clone());
        if let Some(id) = opt_id {
            let nav_focus = self.nav_focus.clone();
            self.popups.push((id, popup, nav_focus));
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
    pub fn close_window(&mut self, id: WindowId, restore_focus: bool) {
        if let Some(index) =
            self.popups
                .iter()
                .enumerate()
                .find_map(|(i, p)| if p.0 == id { Some(i) } else { None })
        {
            let mut old_nav_focus = None;
            while self.popups.len() > index {
                let (wid, popup, onf) = self.popups.pop().unwrap();
                self.popup_removed.push((popup.parent, wid));
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

    /// Send [`Event::Update`] to all widgets
    ///
    /// All widgets across all windows will receive [`Event::Update`] with
    /// the given `id` and `payload`.
    #[inline]
    pub fn update_all(&mut self, id: UpdateId, payload: u64) {
        debug!("update_all: id={:?}, payload={}", id, payload);
        self.shell.update_all(id, payload);
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
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Widget::configure`].
    pub fn size_mgr<F: FnMut(SizeMgr) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(&mut |size, _| {
            result = Some(f(SizeMgr::new(size)));
        });
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
    }

    /// Access a [`SetRectMgr`]
    pub fn set_rect_mgr<F: FnMut(&mut SetRectMgr) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(&mut |size, draw_shared| {
            let mut mgr = SetRectMgr::new(size, draw_shared, self.state);
            result = Some(f(&mut mgr));
        });
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared<F: FnMut(&mut dyn DrawShared) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(&mut |_, draw_shared| {
            result = Some(f(draw_shared));
        });
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
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
    /// [`EventState::set_grab_depress`] to update the grab's depress target.
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
                    pan_grab = self.set_pan_on(id.clone(), mode, false, coord);
                }
                trace!("EventMgr: start mouse grab by {}", start_id);
                self.mouse_grab = Some(MouseGrab {
                    button,
                    repetitions,
                    start_id: start_id.clone(),
                    cur_id: Some(start_id),
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
                    pan_grab = self.set_pan_on(id.clone(), mode, true, coord);
                }
                trace!("EventMgr: start touch grab by {}", start_id);
                self.touch_grab.push(TouchGrab {
                    id: touch_id,
                    start_id,
                    depress: Some(id.clone()),
                    cur_id: Some(id),
                    last_move: coord,
                    coord,
                    mode,
                    pan_grab,
                });
            }
        }

        self.send_action(TkAction::REDRAW);
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
        if id == self.mouse_grab.as_ref().map(|grab| &grab.start_id) {
            self.remove_mouse_grab();
        }
        self.touch_grab.retain(|grab| id != grab.start_id);

        self.grab_press(id, source, coord, GrabMode::Grab, cursor);
    }

    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`EventMgr::grab_press`]). The cursor will be reset when the mouse-grab
    /// ends.
    pub fn update_grab_cursor(&mut self, id: WidgetId, icon: CursorIcon) {
        if let Some(ref grab) = self.mouse_grab {
            if grab.start_id == id {
                self.shell.set_cursor_icon(icon);
            }
        }
    }

    /// Advance the keyboard navigation focus
    ///
    /// This is a shim around [`SetRectMgr::next_nav_focus`].
    #[inline]
    pub fn next_nav_focus(
        &mut self,
        widget: &mut dyn Widget,
        reverse: bool,
        key_focus: bool,
    ) -> bool {
        self.set_rect_mgr(|mgr| mgr.next_nav_focus(widget, reverse, key_focus))
    }
}
