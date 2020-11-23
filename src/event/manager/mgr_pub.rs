// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use log::{debug, trace, warn};
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::draw::SizeHandle;
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
    pub fn char_focus(&self, w_id: WidgetId) -> (bool, bool) {
        if let Some(id) = self.sel_focus {
            if id == w_id {
                return (self.char_focus, true);
            }
        }
        (false, false)
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
        for grab in self.touch_grab.values() {
            if grab.depress == Some(w_id) {
                return true;
            }
        }
        false
    }
}

/// Public API (around toolkit functionality)
impl<'a> Manager<'a> {
    /// Get the current modifier state
    #[inline]
    pub fn modifiers(&self) -> ModifiersState {
        self.mgr.modifiers
    }

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
    /// This may be called from [`WidgetConfig::configure`] or from an event
    /// handler. Note that previously-scheduled updates are cleared when
    /// widgets are reconfigured.
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
    ///
    /// Currently the entire window is redrawn on any redraw request and the
    /// [`WidgetId`] is ignored. In the future partial redraws may be used.
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
    /// A pop-up may be closed by calling [`Manager::close_window`] with
    /// the [`WindowId`] returned by this method.
    #[inline]
    pub fn add_popup(&mut self, popup: kas::Popup) -> WindowId {
        let id = self.tkw.add_popup(popup.clone());
        self.mgr.new_popups.push(popup.id);
        self.mgr.popups.push((id, popup));
        self.mgr.nav_focus = None;
        self.mgr.nav_stack.clear();
        id
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
    pub fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId {
        self.tkw.add_window(widget)
    }

    /// Close a window or pop-up
    #[inline]
    pub fn close_window(&mut self, id: WindowId) {
        if let Some(index) =
            self.mgr.popups.iter().enumerate().find_map(
                |(i, p)| {
                    if p.0 == id {
                        Some(i)
                    } else {
                        None
                    }
                },
            )
        {
            let (_, popup) = self.mgr.popups.remove(index);
            self.mgr.popup_removed.push((popup.parent, id));

            if self.mgr.nav_focus.is_some() {
                // We guess that the parent supports key_nav:
                self.mgr.nav_focus = Some(popup.parent);
                self.mgr.nav_stack.clear();
            }
        }

        // For popups, we need to update mouse/keyboard focus.
        // (For windows, focus gained/lost events do this job.)
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
    pub fn get_clipboard(&mut self) -> Option<String> {
        self.tkw.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard<'c>(&mut self, content: std::borrow::Cow<'c, str>) {
        self.tkw.set_clipboard(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnMut(&mut dyn ThemeApi) -> ThemeAction>(&mut self, mut f: F) {
        self.tkw.adjust_theme(&mut f);
    }

    /// Access a [`SizeHandle`]
    pub fn size_handle<F: FnMut(&mut dyn SizeHandle) -> T, T>(&mut self, mut f: F) -> T {
        let mut result = None;
        self.tkw.size_handle(&mut |size_handle| {
            result = Some(f(size_handle));
        });
        result.expect("TkWindow::size_handle_dyn impl failed to call function argument")
    }
}

/// Public API (around event manager state)
impl<'a> Manager<'a> {
    /// Attempts to set a fallback to receive [`Event::Control`]
    ///
    /// In case a navigation key is pressed (see [`ControlKey`]) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::Control`]. (This does not include
    /// [`Event::Activate`].)
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow [`kas::widget::ScrollRegion`] to
    /// respond to navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: WidgetId) {
        if self.mgr.nav_fallback.is_none() {
            debug!("Manager: nav_fallback = {}", id);
            self.mgr.nav_fallback = Some(id);
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
        self.mgr.accel_stack.push((alt_bypass, HashMap::new()));
    }

    /// Enable `alt_bypass` for the current layer
    ///
    /// This may be called by a child widget during configure, e.g. to enable
    /// alt-bypass for the base layer. See also [`Manager::push_accel_layer`].
    pub fn enable_alt_bypass(&mut self, alt_bypass: bool) {
        if let Some(layer) = self.mgr.accel_stack.last_mut() {
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
        if let Some(layer) = self.mgr.accel_stack.pop() {
            self.mgr.accel_layers.insert(id, layer);
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
    /// see [`kas::text::AccelString`].
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
        if !self.read_only {
            if let Some(last) = self.mgr.accel_stack.last_mut() {
                for key in keys {
                    last.1.insert(*key, id);
                }
            }
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
        if self.read_only {
            return false;
        }

        let start_id = id;
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button, repetitions) => {
                if self.mgr.mouse_grab.is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.mgr.set_pan_on(id, mode, false, coord);
                }
                trace!("Manager: start mouse grab by {}", start_id);
                self.mgr.mouse_grab = Some(MouseGrab {
                    button,
                    repetitions,
                    start_id,
                    depress: Some(id),
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
                self.mgr.touch_grab.insert(
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

        if self.mgr.char_focus && self.mgr.sel_focus != Some(id) {
            self.set_char_focus(None);
        }
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
            PressSource::Mouse(_, _) => {
                if let Some(grab) = self.mgr.mouse_grab.as_mut() {
                    grab.depress = target;
                }
            }
            PressSource::Touch(id) => {
                if let Some(grab) = self.mgr.touch_grab.get_mut(&id) {
                    grab.depress = target;
                }
            }
        }
        self.mgr.send_action(TkAction::Redraw);
    }

    /// Get the current keyboard navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    pub fn nav_focus(&self) -> Option<WidgetId> {
        self.mgr.nav_focus
    }

    /// Clear keyboard navigation focus
    pub fn clear_nav_focus(&mut self) {
        if let Some(id) = self.mgr.nav_focus {
            self.redraw(id);
        }
        self.mgr.nav_focus = None;
        self.mgr.nav_stack.clear();
        trace!("Manager: nav_focus = None");
    }

    /// Set the keyboard navigation focus directly
    ///
    /// [`WidgetConfig::key_nav`] *should* return true for the given widget,
    /// otherwise navigation behaviour may not be correct.
    pub fn set_nav_focus(&mut self, id: WidgetId) {
        self.mgr.nav_focus = Some(id);
        self.mgr.nav_stack.clear();
    }

    /// Advance the keyboard navigation focus
    ///
    /// If some widget currently has nav focus, this will give focus to the next
    /// (or previous) widget under `widget` where [`WidgetConfig::key_nav`]
    /// returns true; otherwise this will give focus to the first (or last)
    /// such widget.
    ///
    /// This method returns true when the navigation focus has been updated,
    /// otherwise leaves the focus unchanged. The caller may (optionally) choose
    /// to call [`Manager::clear_nav_focus`] when this method returns false.
    pub fn next_nav_focus(&mut self, mut widget: &dyn WidgetConfig, reverse: bool) -> bool {
        type WidgetStack<'b> = SmallVec<[&'b dyn WidgetConfig; 16]>;
        let mut widget_stack = WidgetStack::new();

        if let Some(id) = self.mgr.popups.last().map(|(_, p)| p.id) {
            if let Some(w) = widget.find(id) {
                widget = w;
            } else {
                // This is a corner-case. Do nothing.
                return false;
            }
        }

        if self.mgr.nav_stack.is_empty() {
            if let Some(id) = self.mgr.nav_focus {
                // This is caused by set_nav_focus; we need to rebuild nav_stack
                'l: while id != widget.id() {
                    for index in 0..widget.len() {
                        let w = widget.get(index).unwrap();
                        if w.is_ancestor_of(id) {
                            self.mgr.nav_stack.push(index as u32);
                            widget_stack.push(widget);
                            widget = w;
                            continue 'l;
                        }
                    }

                    warn!("next_nav_focus: unable to find widget {}", id);
                    self.mgr.nav_focus = None;
                    self.mgr.nav_stack.clear();
                    return false;
                }
            }
        } else if self
            .mgr
            .nav_focus
            .map(|id| !widget.is_ancestor_of(id))
            .unwrap_or(true)
        {
            self.mgr.nav_stack.clear();
        } else {
            // Reconstruct widget_stack:
            for index in self.mgr.nav_stack.iter().cloned() {
                let new = widget.get(index as usize).unwrap();
                widget_stack.push(widget);
                widget = new;
            }
        }

        // Progresses to the first child (or last if reverse).
        // Returns true if a child is found.
        // Breaks to given lifetime on error.
        macro_rules! do_child {
            ($lt:lifetime, $nav_stack:ident, $widget:ident, $widget_stack:ident) => {{
                let range = $widget.spatial_range();
                if $widget.is_disabled() || range.1 == std::usize::MAX {
                    false
                } else {
                    // We have a child; the first is range.0 unless reverse
                    let index = match reverse {
                        false => range.0,
                        true => range.1,
                    };
                    let new = match $widget.get(index) {
                        None => break $lt,
                        Some(w) => w,
                    };
                    $nav_stack.push(index as u32);
                    $widget_stack.push($widget);
                    $widget = new;
                    true
                }
            }};
        };

        // Progresses to the next (or previous) sibling, otherwise pops to the
        // parent. Returns true if a sibling is found.
        // Breaks to given lifetime on error.
        macro_rules! do_sibling_or_pop {
            ($lt:lifetime, $nav_stack:ident, $widget:ident, $widget_stack:ident) => {{
                let mut index;
                match ($nav_stack.pop(), $widget_stack.pop()) {
                    (Some(i), Some(w)) => {
                        index = i as usize;
                        $widget = w;
                    }
                    _ => break $lt,
                };
                let mut range = $widget.spatial_range();
                if $widget.is_disabled() || range.1 == std::usize::MAX {
                    break $lt;
                }

                let reverse = (range.1 < range.0) ^ reverse;
                if range.1 < range.0 {
                    std::mem::swap(&mut range.0, &mut range.1);
                }

                // Look for next sibling
                let have_sibling = match reverse {
                    false if index < range.1 => {
                        index += 1;
                        true
                    }
                    true if range.0 < index => {
                        index -= 1;
                        true
                    }
                    _ => false,
                };

                if have_sibling {
                    let new = match $widget.get(index) {
                        None => break $lt,
                        Some(w) => w,
                    };
                    $nav_stack.push(index as u32);
                    $widget_stack.push($widget);
                    $widget = new;
                }
                have_sibling
            }};
        };

        macro_rules! try_set_focus {
            ($self:ident, $widget:ident) => {
                if $widget.key_nav() && !$widget.is_disabled() {
                    $self.mgr.nav_focus = Some($widget.id());
                    trace!("Manager: nav_focus = {:?}", $self.mgr.nav_focus);
                    return true;
                }
            };
        }

        // We redraw in all cases. Since this is not part of widget event
        // processing, we can push directly to self.mgr.action.
        self.mgr.send_action(TkAction::Redraw);
        let nav_stack = &mut self.mgr.nav_stack;

        if !reverse {
            // Depth-first search without function recursion. Our starting
            // entry has already been used (if applicable); the next
            // candidate is its first child.
            'l1: loop {
                if do_child!('l1, nav_stack, widget, widget_stack) {
                    try_set_focus!(self, widget);
                    continue;
                }

                loop {
                    if do_sibling_or_pop!('l1, nav_stack, widget, widget_stack) {
                        try_set_focus!(self, widget);
                        break;
                    }
                }
            }
        } else {
            // Reverse depth-first search
            let mut start = self.mgr.nav_focus.is_none();
            'l2: loop {
                if start || do_sibling_or_pop!('l2, nav_stack, widget, widget_stack) {
                    start = false;
                    while do_child!('l2, nav_stack, widget, widget_stack) {}
                }

                try_set_focus!(self, widget);
            }
        }

        false
    }
}
