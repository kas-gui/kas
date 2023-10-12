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
#[cfg(all(wayland_platform, feature = "clipboard"))]
use crate::util::warn_about_error;
use crate::{Action, Erased, HasId, Id, Window, WindowId};
#[allow(unused)] use crate::{Events, Layout}; // for doc-links

/// Public API
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

    /// True when access key labels should be shown
    ///
    /// (True when Alt is held and no widget has character focus.)
    ///
    /// This is a fast check.
    #[inline]
    pub fn show_access_labels(&self) -> bool {
        self.modifiers.alt_key()
    }

    /// Get whether this widget has `(key_focus, sel_focus)`
    ///
    /// -   `key_focus`: implies this widget receives keyboard input
    /// -   `sel_focus`: implies this widget is allowed to select things
    ///
    /// Note that `key_focus` implies `sel_focus`.
    #[inline]
    pub fn has_key_focus(&self, w_id: &Id) -> (bool, bool) {
        let sel_focus = *w_id == self.sel_focus;
        (sel_focus && self.key_focus, sel_focus)
    }

    /// Get whether this widget has navigation focus
    #[inline]
    pub fn has_nav_focus(&self, w_id: &Id) -> bool {
        *w_id == self.nav_focus
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_hovered(&self, w_id: &Id) -> bool {
        self.mouse_grab.is_none() && *w_id == self.hover
    }

    /// Get whether widget `id` or any of its descendants are under the mouse cursor
    #[inline]
    pub fn is_hovered_recursive(&self, id: &Id) -> bool {
        self.mouse_grab.is_none()
            && self
                .hover
                .as_ref()
                .map(|h| id.is_ancestor_of(h))
                .unwrap_or(false)
    }

    /// Check whether the given widget is visually depressed
    pub fn is_depressed(&self, w_id: &Id) -> bool {
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
    pub fn is_disabled(&self, w_id: &Id) -> bool {
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
    /// events ([`Unused`] is returned automatically when the
    /// recipient or any ancestor is disabled).
    pub fn set_disabled(&mut self, w_id: Id, state: bool) {
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
            self.action(&w_id, Action::REDRAW);
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
    pub fn request_timer_update(&mut self, id: Id, payload: u64, delay: Duration, first: bool) {
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
        self.action |= Action::EXIT;
    }

    /// Notify that a [`Action`] action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn action(&mut self, id: impl HasId, action: Action) {
        fn inner(cx: &mut EventState, id: Id, mut action: Action) {
            const RE_UP: Action = Action::RECONFIGURE.union(Action::UPDATE);
            if !(action & RE_UP).is_empty() {
                cx.request_update(id, action.contains(Action::RECONFIGURE));
                action.remove(RE_UP);
            }

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

    /// Attempts to set a fallback to receive [`Event::Command`]
    ///
    /// In case a navigation key is pressed (see [`Command`]) but no widget has
    /// navigation focus, then, if a fallback has been set, that widget will
    /// receive the key via [`Event::Command`].
    ///
    /// Only one widget can be a fallback, and the *first* to set itself wins.
    /// This is primarily used to allow scroll-region widgets to
    /// respond to navigation keys when no widget has focus.
    pub fn register_nav_fallback(&mut self, id: Id) {
        if self.nav_fallback.is_none() {
            log::debug!(target: "kas_core::event","register_nav_fallback: id={id}");
            self.nav_fallback = Some(id);
        }
    }

    fn access_layer_for_id(&mut self, id: &Id) -> Option<&mut AccessLayer> {
        let root = &Id::ROOT;
        for (k, v) in self.access_layers.range_mut(root..=id).rev() {
            if k.is_ancestor_of(id) {
                return Some(v);
            };
        }
        debug_assert!(false, "expected ROOT access layer");
        None
    }

    /// Add a new access key layer
    ///
    /// This method constructs a new "layer" for access keys: any keys
    /// added via [`EventState::add_access_key`] to a widget which is a descentant
    /// of (or equal to) `id` will only be active when that layer is active.
    ///
    /// This method should only be called by parents of a pop-up: layers over
    /// the base layer are *only* activated by an open pop-up.
    ///
    /// If `alt_bypass` is true, then this layer's access keys will be
    /// active even without Alt pressed (but only highlighted with Alt pressed).
    pub fn new_access_layer(&mut self, id: Id, alt_bypass: bool) {
        self.access_layers.insert(id, (alt_bypass, HashMap::new()));
    }

    /// Enable `alt_bypass` for layer
    ///
    /// This may be called by a child widget during configure to enable or
    /// disable alt-bypass for the access-key layer containing its access keys.
    /// This allows access keys to be used as shortcuts without the Alt
    /// key held. See also [`EventState::new_access_layer`].
    pub fn enable_alt_bypass(&mut self, id: &Id, alt_bypass: bool) {
        if let Some(layer) = self.access_layer_for_id(id) {
            layer.0 = alt_bypass;
        }
    }

    /// Adds an access key for a widget
    ///
    /// An *access key* (also known as mnemonic) is a shortcut key able to
    /// directly open menus,
    /// activate buttons, etc. A user triggers the key by pressing `Alt+Key`,
    /// or (if `alt_bypass` is enabled) by simply pressing the key.
    /// The widget with this `id` then receives [`Command::Activate`].
    ///
    /// Note that access keys may be automatically derived from labels:
    /// see [`crate::text::AccessString`].
    ///
    /// Access keys are added to the layer with the longest path which is
    /// an ancestor of `id`. This usually means that if the widget is part of a
    /// pop-up, the key is only active when that pop-up is open.
    /// See [`EventState::new_access_layer`].
    ///
    /// This should only be called from [`Events::configure`].
    #[inline]
    pub fn add_access_key(&mut self, id: &Id, key: Key) {
        if let Some(layer) = self.access_layer_for_id(id) {
            layer.1.entry(key).or_insert_with(|| id.clone());
        }
    }

    /// Visually depress a widget via a key code
    ///
    /// When a button-like widget is activated by a key it may call this to
    /// ensure the widget is visually depressed until the key is released.
    /// The widget will not receive a notification of key-release but will be
    /// redrawn automatically.
    ///
    /// Note that keyboard shortcuts and mnemonics should usually match against
    /// the "logical key". [`KeyCode`] is used here since the the logical key
    /// may be changed by modifier keys.
    pub fn depress_with_key(&mut self, id: Id, code: KeyCode) {
        if self.key_depress.values().any(|v| *v == id) {
            return;
        }

        self.key_depress.insert(code, id.clone());
        self.action(id, Action::REDRAW);
    }

    /// Request keyboard input focus
    ///
    /// When granted, the widget will receive [`Event::KeyFocus`] followed by
    /// [`Event::Key`] for each key press / release. Note that this disables
    /// translation of key events to [`Event::Command`] while key focus is
    /// active.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    ///
    /// Key focus implies sel focus (see [`Self::request_sel_focus`]) and
    /// navigation focus.
    #[inline]
    pub fn request_key_focus(&mut self, target: Id, source: FocusSource) {
        self.pending_sel_focus = Some(PendingSelFocus {
            target,
            key_focus: true,
            source,
        });
    }

    /// Request selection focus
    ///
    /// To prevent multiple simultaneous selections (e.g. of text) in the UI,
    /// only widgets with "selection focus" are allowed to select things.
    /// Selection focus is implied by character focus. [`Event::LostSelFocus`]
    /// is sent when selection focus is lost; in this case any existing
    /// selection should be cleared.
    ///
    /// The `source` parameter is used by [`Event::SelFocus`].
    ///
    /// Selection focus implies navigation focus.
    ///
    /// When key focus is lost, [`Event::LostSelFocus`] is sent.
    #[inline]
    pub fn request_sel_focus(&mut self, target: Id, source: FocusSource) {
        if let Some(ref pending) = self.pending_sel_focus {
            if pending.target == target {
                return;
            }
        }

        self.pending_sel_focus = Some(PendingSelFocus {
            target,
            key_focus: false,
            source,
        });
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
    pub fn set_grab_depress(&mut self, source: PressSource, target: Option<Id>) -> bool {
        let mut old = None;
        let mut redraw = false;
        match source {
            PressSource::Mouse(_, _) => {
                if let Some(grab) = self.mouse_grab.as_mut() {
                    redraw = grab.depress != target;
                    old = grab.depress.take();
                    grab.depress = target.clone();
                }
            }
            PressSource::Touch(id) => {
                if let Some(grab) = self.get_touch(id) {
                    redraw = grab.depress != target;
                    old = grab.depress.take();
                    grab.depress = target.clone();
                }
            }
        }
        if redraw {
            log::trace!(target: "kas_core::event", "set_grab_depress: target={target:?}");
            self.opt_action(old, Action::REDRAW);
            self.opt_action(target, Action::REDRAW);
        }
        redraw
    }

    /// Returns true if `id` or any descendant has a mouse or touch grab
    pub fn any_pin_on(&self, id: &Id) -> bool {
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

    /// Get the current navigation focus, if any
    ///
    /// This is the widget selected by navigating the UI with the Tab key.
    ///
    /// Note: changing navigation focus (e.g. via [`Self::clear_nav_focus`],
    /// [`Self::set_nav_focus`] or [`Self::next_nav_focus`]) does not
    /// immediately affect the result of this method.
    #[inline]
    pub fn nav_focus(&self) -> Option<&Id> {
        self.nav_focus.as_ref()
    }

    /// Clear navigation focus
    pub fn clear_nav_focus(&mut self) {
        self.pending_nav_focus = PendingNavFocus::Set {
            target: None,
            source: FocusSource::Synthetic,
        };
    }

    /// Set navigation focus directly
    ///
    /// If `id` already has navigation focus or navigation focus is disabled
    /// globally then nothing happens, otherwise widget `id` should receive
    /// [`Event::NavFocus`].
    ///
    /// Normally, [`Events::navigable`] will be true for widget `id` but this
    /// is not checked or required. For example, a `ScrollLabel` can receive
    /// focus on text selection with the mouse.
    pub fn set_nav_focus(&mut self, id: Id, source: FocusSource) {
        self.pending_nav_focus = PendingNavFocus::Set {
            target: Some(id),
            source,
        };
    }

    /// Advance the navigation focus
    ///
    /// If `target == Some(id)`, this looks for the next widget from `id`
    /// (inclusive) which is navigable ([`Events::navigable`]). Otherwise where
    /// some widget `id` has [`nav_focus`](Self::nav_focus) this looks for the
    /// next navigable widget *excluding* `id`. If no reference is available,
    /// this instead looks for the first navigable widget.
    ///
    /// If `reverse`, instead search for the previous or last navigable widget.
    pub fn next_nav_focus(
        &mut self,
        target: impl Into<Option<Id>>,
        reverse: bool,
        source: FocusSource,
    ) {
        self.pending_nav_focus = PendingNavFocus::Next {
            target: target.into(),
            reverse,
            source,
        };
    }

    /// Set the cursor icon
    ///
    /// This is normally called when handling [`Event::MouseHover`]. In other
    /// cases, calling this method may be ineffective. The cursor is
    /// automatically "unset" when the widget is no longer hovered.
    ///
    /// See also [`EventCx::set_grab_cursor`]: if a mouse grab
    /// ([`Press::grab`]) is active, its icon takes precedence.
    pub fn set_hover_cursor(&mut self, icon: CursorIcon) {
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
    pub fn push_async<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        M: Debug + 'static,
    {
        self.push_async_erased(id, async { Erased::new(fut.await) });
    }

    /// Asynchronously push a type-erased message to the stack via a [`Future`]
    ///
    /// This is a low-level variant of [`Self::push_async`].
    pub fn push_async_erased<Fut>(&mut self, id: Id, fut: Fut)
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
    pub fn push_spawn<Fut, M>(&mut self, id: Id, fut: Fut)
    where
        Fut: IntoFuture<Output = M> + 'static,
        Fut::IntoFuture: Send,
        M: Debug + Send + 'static,
    {
        self.push_async(id, async_global_executor::spawn(fut.into_future()));
    }

    /// Request update and optionally configure to widget `id`
    ///
    /// If `reconf`, widget `id` will be [configured](Events::configure)
    /// (includes update), otherwise `id` will only be [updated](Events::update).
    pub fn request_update(&mut self, id: Id, reconf: bool) {
        self.pending_update = if let Some((id2, rc2)) = self.pending_update.take() {
            Some((id.common_ancestor(&id2), reconf || rc2))
        } else {
            Some((id, reconf))
        };
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

    /// Send a command to a widget
    ///
    /// Sends [`Event::Command`] to widget `id`. The event is queued to send
    /// later, thus any actions by the receiving widget will not be immediately
    /// visible to the caller of this method.
    ///
    /// When calling this method, be aware that some widgets use an inner
    /// component to handle events, thus calling with the outer widget's `id`
    /// may not have the desired effect. [`Layout::find_id`] and
    /// [`EventState::next_nav_focus`] are usually able to find the appropriate
    /// event-handling target.
    pub fn send_command(&mut self, id: Id, cmd: Command) {
        self.pending_cmds.push_back((id, cmd));
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
    /// The popup automatically receives mouse-motion events
    /// ([`Event::CursorMove`]) which may be used to navigate menus.
    /// The parent automatically receives the "depressed" visual state.
    ///
    /// It is recommended to call [`EventState::set_nav_focus`] or
    /// [`EventState::next_nav_focus`] after this method.
    ///
    /// A pop-up may be closed by calling [`EventCx::close_window`] with
    /// the [`WindowId`] returned by this method.
    pub(crate) fn add_popup(&mut self, popup: crate::PopupDescriptor) -> WindowId {
        log::trace!(target: "kas_core::event", "add_popup: {popup:?}");

        let parent_id = self.window.window_id();
        let id = self.shell.add_popup(parent_id, popup.clone());
        let nav_focus = self.nav_focus.clone();
        self.popups.push((id, popup, nav_focus));
        self.clear_nav_focus();
        id
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
    /// Navigation focus will return to whichever widget had focus before
    /// the popup was open.
    pub fn close_window(&mut self, id: WindowId) {
        if let Some(index) =
            self.popups
                .iter()
                .enumerate()
                .find_map(|(i, p)| if p.0 == id { Some(i) } else { None })
        {
            let (wid, popup, onf) = self.popups.remove(index);
            self.popup_removed.push((popup.id, wid));
            self.shell.close_window(wid);

            if let Some(id) = onf {
                self.set_nav_focus(id, FocusSource::Synthetic);
            }
            return;
        }

        self.shell.close_window(id);
    }

    /// Enable window dragging for current click
    ///
    /// This calls [`winit::window::Window::drag_window`](https://docs.rs/winit/latest/winit/window/struct.Window.html#method.drag_window). Errors are ignored.
    pub fn drag_window(&self) {
        #[cfg(winit)]
        if let Some(ww) = self.window.winit_window() {
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
        if let Some(ww) = self.window.winit_window() {
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

        self.shell.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            cb.store(content);
            return;
        }

        self.shell.set_clipboard(content)
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
    #[inline]
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

        self.shell.get_primary()
    }

    /// Set contents of primary buffer
    ///
    /// Linux has a "primary buffer" with implicit copy on text selection and
    /// paste on middle-click. This method does nothing on other platforms.
    #[inline]
    pub fn set_primary(&mut self, content: String) {
        #[cfg(all(wayland_platform, feature = "clipboard"))]
        if let Some(cb) = self.window.wayland_clipboard() {
            cb.store_primary(content);
            return;
        }

        self.shell.set_primary(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnOnce(&mut dyn ThemeControl) -> Action>(&mut self, f: F) {
        self.shell.adjust_theme(Box::new(f));
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
        self.shell.draw_shared()
    }

    /// Directly access Winit Window
    ///
    /// This is a temporary API, allowing e.g. to minimize the window.
    #[cfg(winit)]
    pub fn winit_window(&self) -> Option<&winit::window::Window> {
        self.window.winit_window()
    }

    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`Press::grab`]). The cursor will be reset when the mouse-grab
    /// ends.
    pub fn set_grab_cursor(&mut self, id: &Id, icon: CursorIcon) {
        if let Some(ref grab) = self.mouse_grab {
            if grab.start_id == *id {
                self.window.set_cursor_icon(icon);
            }
        }
    }
}
