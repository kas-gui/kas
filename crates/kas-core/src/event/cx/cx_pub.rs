// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — public API

use std::fmt::Debug;
use std::future::IntoFuture;
use std::time::{Duration, Instant};

use super::*;
use crate::cast::Conv;
use crate::draw::DrawShared;
use crate::event::config::ChangeConfig;
use crate::geom::{Offset, Vec2};
use crate::theme::{SizeCx, ThemeControl};
use crate::{Action, Erased, WidgetId, Window, WindowId};
#[allow(unused)] use crate::{Events, Layout}; // for doc-links

impl<'a> std::ops::BitOrAssign<Action> for EventCx<'a> {
    #[inline]
    fn bitor_assign(&mut self, action: Action) {
        self.send_action(action);
    }
}

impl std::ops::BitOrAssign<Action> for EventState {
    #[inline]
    fn bitor_assign(&mut self, action: Action) {
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
        self.modifiers.alt_key()
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

    /// Get whether widget `id` or any of its descendants are under the mouse cursor
    #[inline]
    pub fn is_hovered_recursive(&self, id: &WidgetId) -> bool {
        self.mouse_grab.is_none()
            && self
                .hover
                .as_ref()
                .map(|h| id.is_ancestor_of(h))
                .unwrap_or(false)
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
        Vec2::conv(dist).abs().max_comp() >= self.config.pan_dist_thresh()
    }

    /// Update event configuration
    #[inline]
    pub fn change_config(&mut self, msg: ChangeConfig) {
        match self.config.config.try_borrow_mut() {
            Ok(mut config) => {
                config.change_config(msg);
                self.action |= Action::EVENT_CONFIG;
            }
            Err(_) => log::error!("EventState::change_config: failed to mutably borrow config"),
        }
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
            self.send_action(Action::REDRAW);
            self.disabled.push(w_id);
        }
    }

    /// Schedule an update
    ///
    /// Widget updates may be used for animation and timed responses. See also
    /// [`Draw::animate`](crate::draw::Draw::animate) for animation.
    ///
    /// Widget `id` will receive [`Event::TimerUpdate`] with this `payload` at
    /// approximately `time = now + delay` (or possibly a little later due to
    /// frame-rate limiters and processing time).
    ///
    /// Requesting an update with `delay == 0` is valid, except from an
    /// [`Event::TimerUpdate`] handler (where it may cause an infinite loop).
    ///
    /// If multiple updates with the same `id` and `payload` are requested,
    /// these are merged (using the earliest time if `first` is true).
    pub fn request_timer_update(
        &mut self,
        id: WidgetId,
        payload: u64,
        delay: Duration,
        first: bool,
    ) {
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
            log::trace!(
                target: "kas_core::event",
                "request_timer_update: update {id} at now+{}ms",
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
    /// equivalent to: `cx.send_action(Action::REDRAW);`
    #[inline]
    pub fn redraw(&mut self, _id: WidgetId) {
        // Theoretically, notifying by WidgetId allows selective redrawing
        // (damage events). This is not yet implemented.
        self.send_action(Action::REDRAW);
    }

    /// Notify that a [`Action`] action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Calling `cx.send_action(action)` is equivalent to `*cx |= action`.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn send_action(&mut self, action: Action) {
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
            log::debug!(target: "kas_core::event","register_nav_fallback: id={id}");
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
    /// added via [`EventState::add_accel_key`] to a widget which is a descentant
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
    /// This should only be called from [`Events::configure`].
    #[inline]
    pub fn add_accel_key(&mut self, id: &WidgetId, key: Key) {
        if let Some(layer) = self.accel_layer_for_id(id) {
            layer.1.entry(key).or_insert_with(|| id.clone());
        }
    }

    /// Request character-input focus
    ///
    /// Returns true on success or when the widget already had char focus.
    ///
    /// Character data is sent to the widget with char focus via
    /// [`Event::Text`] and [`Event::Command`].
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
    /// ([`Press::grab`]), the widget owning the grab may set itself
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
            log::trace!(target: "kas_core::event", "set_grab_depress: target={target:?}");
            self.send_action(Action::REDRAW);
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
            self.send_action(Action::REDRAW);
            self.pending
                .push_back(Pending::Send(id, Event::LostNavFocus));
        }
        self.clear_char_focus();
        log::trace!(target: "kas_core::event", "clear_nav_focus");
    }

    /// Set the keyboard navigation focus directly
    ///
    /// Normally, [`Events::navigable`] will be true for the specified
    /// widget, but this is not required, e.g. a `ScrollLabel` can receive focus
    /// on text selection with the mouse. (Currently such widgets will receive
    /// events like any other with nav focus, but this may change.)
    ///
    /// The target widget, if not already having navigation focus, will receive
    /// [`Event::NavFocus`] with `key_focus` as the payload. This boolean should
    /// be true if focussing in response to keyboard input, false if reacting to
    /// mouse or touch input.
    pub fn set_nav_focus(&mut self, id: WidgetId, key_focus: bool) {
        if id == self.nav_focus || !self.config.nav_focus {
            return;
        }

        self.send_action(Action::REDRAW);
        if let Some(old_id) = self.nav_focus.take() {
            self.pending
                .push_back(Pending::Send(old_id, Event::LostNavFocus));
        }
        if id != self.sel_focus {
            self.clear_char_focus();
        }
        self.nav_focus = Some(id.clone());
        log::trace!(target: "kas_core::event", "set_nav_focus: {id}");
        self.pending
            .push_back(Pending::Send(id, Event::NavFocus(key_focus)));
    }

    /// Advance the keyboard navigation focus
    ///
    /// If `target == Some(id)`, this looks for the next widget from `id`
    /// (inclusive) which is navigable ([`Events::navigable`]). Otherwise where
    /// some widget `id` has [`nav_focus`](Self::nav_focus) this looks for the
    /// next navigable widget *excluding* `id`. If no reference is available,
    /// this instead looks for the first navigable widget.
    ///
    /// If `reverse`, instead search for the previous or last navigable widget.
    ///
    /// Parameter `key_focus` should be `true` for keyboard-driven and
    /// programmatic navigation, `false` for mouse/touch-driven navigation.
    /// The parameter is passed to the target through [`Event::NavFocus`].
    pub fn next_nav_focus(
        &mut self,
        target: impl Into<Option<WidgetId>>,
        reverse: bool,
        key_focus: bool,
    ) {
        self.pending.push_back(Pending::NextNavFocus {
            target: target.into(),
            reverse,
            key_focus,
        });
    }

    /// Set the cursor icon
    ///
    /// This is normally called when handling [`Event::MouseHover`]. In other
    /// cases, calling this method may be ineffective. The cursor is
    /// automatically "unset" when the widget is no longer hovered.
    ///
    /// If a mouse grab ([`Press::grab`]) is active, its icon takes precedence.
    pub fn set_cursor_icon(&mut self, icon: CursorIcon) {
        // Note: this is acted on by EventState::update
        self.hover_icon = icon;
    }

    /// Asynchronously push a message to the stack via a [`Future`]
    ///
    /// The future is polled after event handling and after drawing and is able
    /// to wake the event loop. This future is executed on the main thread; for
    /// high-CPU tasks use [`Self::push_spawn`] instead.
    ///
    /// The future must resolve to a message on completion. This message is
    /// pushed to the message stack as if it were pushed with [`EventCx::push`]
    /// from widget `id`, allowing this widget or any ancestor to handle it in
    /// [`Events::handle_messages`].
    //
    // TODO: Can we identify the calling widget `id` via the context (EventCx)?
    pub fn push_async<Fut, M>(&mut self, id: WidgetId, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        M: Debug + 'static,
    {
        self.push_async_erased(id, async { Erased::new(fut.await) });
    }

    /// Asynchronously push a type-erased message to the stack via a [`Future`]
    ///
    /// This is a low-level variant of [`Self::push_async`].
    pub fn push_async_erased<Fut>(&mut self, id: WidgetId, fut: Fut)
    where
        Fut: IntoFuture<Output = Erased> + 'static,
    {
        let fut = Box::pin(fut.into_future());
        self.fut_messages.push((id, fut));
    }

    /// Spawn a task, run on a thread pool
    ///
    /// The future is spawned to a thread-pool before the event-handling loop
    /// sleeps, and is able to wake the loop on completion. Tasks involving
    /// significant CPU work should use this method over [`Self::push_async`].
    ///
    /// This method is simply a wrapper around [`async_global_executor::spawn`]
    /// and [`Self::push_async`]; if a different multi-threaded executor is
    /// available, that may be used instead. See also [`async_global_executor`]
    /// documentation of configuration.
    #[cfg(feature = "spawn")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "spawn")))]
    pub fn push_spawn<Fut, M>(&mut self, id: WidgetId, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        Fut::IntoFuture: Send,
        M: Debug + Send + 'static,
    {
        self.push_async(id, async_global_executor::spawn(fut.into_future()));
    }

    /// Request re-configure of widget `id`
    ///
    /// This method requires that `id` is a valid path to an already-configured
    /// widget. E.g. if widget `w` adds a new child, it may call
    /// `request_reconfigure(self.id())` but not
    /// `request_reconfigure(child_id)` (which would cause a panic:
    /// `'WidgetId::next_key_after: invalid'`).
    pub fn request_reconfigure(&mut self, id: WidgetId) {
        self.pending.push_back(Pending::Configure(id));
    }

    /// Request update to widget `id`
    ///
    /// Schedules a call to [`Events::update`] on widget `id`.
    pub fn request_update(&mut self, id: WidgetId) {
        self.pending.push_back(Pending::Update(id));
    }

    /// Request set_rect of the given path
    pub fn request_set_rect(&mut self, id: WidgetId) {
        self.pending.push_back(Pending::SetRect(id));
    }
}

/// Public API
impl<'a> EventCx<'a> {
    /// Configure a widget
    ///
    /// Note that, when handling events, this method returns the *old* state.
    ///
    /// This is a shortcut to [`ConfigCx::configure`].
    #[inline]
    pub fn configure(&mut self, mut widget: Node<'_>, id: WidgetId) {
        self.config_cx(|cx| widget._configure(cx, id));
    }

    /// Update a widget
    ///
    /// [`Events::update`] will be called recursively on each child and finally
    /// `self`. If a widget stores state which it passes to children as input
    /// data, it should call this (or [`ConfigCx::update`]) after mutating the state.
    #[inline]
    pub fn update(&mut self, mut widget: Node<'_>) {
        self.config_cx(|cx| widget._update(cx));
    }

    /// Get the index of the last child visited
    ///
    /// This is only used when unwinding (traversing back up the widget tree),
    /// and returns the index of the child last visited. E.g. when
    /// [`Events::handle_messages`] is called, this method returns the index of
    /// the child which submitted the message (or whose descendant did).
    /// Otherwise this returns `None` (including when the widget itself is the
    /// submitter of the message).
    pub fn last_child(&self) -> Option<usize> {
        self.last_child
    }

    /// Send an event to a widget
    ///
    /// Sends `event` to widget `id`. The event is queued to send later, thus
    /// any actions by the receiving widget will not be immediately visible to
    /// the caller of this method.
    ///
    /// When calling this method, be aware that:
    ///
    /// -   Some widgets use an inner component to handle events, thus calling
    ///     with the outer widget's `id` may not have the desired effect.
    ///     [`Layout::find_id`] and [`EventState::next_nav_focus`] are able to find
    ///     the appropriate event-handling target.
    ///     (TODO: do we need another method to find this target?)
    /// -   Some events such as [`Event::PressMove`] contain embedded widget
    ///     identifiers which may affect handling of the event.
    pub fn send(&mut self, id: WidgetId, event: Event) {
        self.pending.push_back(Pending::Send(id, event));
    }

    /// Push a message to the stack
    ///
    /// The message is first type-erased by wrapping with [`Erased`],
    /// then pushed to the stack.
    ///
    /// The message may be [popped](EventCx::try_pop) or
    /// [observed](EventCx::try_observe) from [`Events::handle_messages`]
    /// by the widget itself, its parent, or any ancestor.
    pub fn push<M: Debug + 'static>(&mut self, msg: M) {
        self.push_erased(Erased::new(msg));
    }

    /// Push a type-erased message to the stack
    ///
    /// This is a lower-level variant of [`Self::push`].
    ///
    /// The message may be [popped](EventCx::try_pop) or
    /// [observed](EventCx::try_observe) from [`Events::handle_messages`]
    /// by the widget itself, its parent, or any ancestor.
    pub fn push_erased(&mut self, msg: Erased) {
        self.messages.push_erased(msg);
    }

    /// True if the message stack is non-empty
    pub fn has_msg(&self) -> bool {
        self.messages.has_any()
    }

    /// Try popping the last message from the stack with the given type
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_pop<M: Debug + 'static>(&mut self) -> Option<M> {
        self.messages.try_pop()
    }

    /// Try observing the last message on the stack without popping
    ///
    /// This method may be called from [`Events::handle_messages`].
    pub fn try_observe<M: Debug + 'static>(&self) -> Option<&M> {
        self.messages.try_observe()
    }

    /// Set a scroll action
    ///
    /// When setting [`Scroll::Rect`], use the widget's own coordinate space.
    ///
    /// Note that calling this method has no effect on the widget itself, but
    /// affects parents via their [`Events::handle_scroll`] method.
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
    /// ([`Event::CursorMove`]) which may be used to navigate menus.
    /// The parent automatically receives the "depressed" visual state.
    ///
    /// It is recommended to call [`EventState::set_nav_focus`] or
    /// [`EventState::next_nav_focus`] after this method.
    ///
    /// A pop-up may be closed by calling [`EventCx::close_window`] with
    /// the [`WindowId`] returned by this method.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    pub fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId> {
        log::trace!(target: "kas_core::event", "add_popup: {popup:?}");
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
    /// starts (see `kas_wgpu::Toolkit::add`), however that method is not
    /// available to a running UI. This method may be used instead.
    ///
    /// Requirement: the type `Data` must match the type of data passed to the
    /// `Shell` and used by other windows. If not, a run-time error will result.
    ///
    /// Caveat: if an error occurs opening the new window it will not be
    /// reported (except via log messages).
    #[inline]
    pub fn add_window<Data: 'static>(&mut self, window: Window<Data>) -> WindowId {
        let data_type_id = std::any::TypeId::of::<Data>();
        unsafe {
            let window: Window<()> = std::mem::transmute(window);
            self.shell.add_window(window, data_type_id)
        }
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

    /// Enable window dragging for current click
    ///
    /// This calls [`winit::window::Window::drag_window`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.drag_window). Errors are ignored.
    pub fn drag_window(&self) {
        #[cfg(winit)]
        if let Some(ww) = self.shell.winit_window() {
            if let Err(e) = ww.drag_window() {
                log::warn!("EventCx::drag_window: {e}");
            }
        }
    }

    /// Enable window resizing for the current click
    ///
    /// This calls [`winit::window::Window::drag_resize_window`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.drag_resize_window). Errors are ignored.
    pub fn drag_resize_window(&self, direction: ResizeDirection) {
        #[cfg(winit)]
        if let Some(ww) = self.shell.winit_window() {
            if let Err(e) = ww.drag_resize_window(direction) {
                log::warn!("EventCx::drag_resize_window: {e}");
            }
        }
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

    /// Get contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    #[inline]
    pub fn get_primary(&mut self) -> Option<String> {
        self.shell.get_primary()
    }

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    #[inline]
    pub fn set_primary(&mut self, content: String) {
        self.shell.set_primary(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnOnce(&mut dyn ThemeControl) -> Action>(&mut self, f: F) {
        self.shell.adjust_theme(Box::new(f));
    }

    /// Access a [`SizeCx`]
    ///
    /// Warning: sizes are calculated using the window's current scale factor.
    /// This may change, even without user action, since some platforms
    /// always initialize windows with scale factor 1.
    /// See also notes on [`Events::configure`].
    pub fn size_cx<F: FnOnce(SizeCx) -> T, T>(&mut self, f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(Box::new(|size, _| {
            result = Some(f(SizeCx::new(size)));
        }));
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
    }

    /// Access a [`ConfigCx`]
    pub fn config_cx<F: FnOnce(&mut ConfigCx) -> T, T>(&mut self, f: F) -> T {
        let mut result = None;
        self.shell
            .size_and_draw_shared(Box::new(|size, draw_shared| {
                let mut cx = ConfigCx::new(size, draw_shared, self.state);
                result = Some(f(&mut cx));
            }));
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
    }

    /// Access a [`DrawShared`]
    pub fn draw_shared<F: FnOnce(&mut dyn DrawShared) -> T, T>(&mut self, f: F) -> T {
        let mut result = None;
        self.shell.size_and_draw_shared(Box::new(|_, draw_shared| {
            result = Some(f(draw_shared));
        }));
        result.expect("ShellWindow::size_and_draw_shared impl failed to call function argument")
    }

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    #[cfg(winit)]
    pub fn winit_window(&self) -> Option<&winit::window::Window> {
        self.shell.winit_window()
    }

    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`Press::grab`]). The cursor will be reset when the mouse-grab
    /// ends.
    pub fn update_grab_cursor(&mut self, id: WidgetId, icon: CursorIcon) {
        if let Some(ref grab) = self.mouse_grab {
            if grab.start_id == id {
                self.shell.set_cursor_icon(icon);
            }
        }
    }
}
