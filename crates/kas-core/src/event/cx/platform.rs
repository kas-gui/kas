// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — platform API

use super::*;
#[cfg(feature = "accesskit")] use crate::cast::CastApprox;
use crate::theme::ThemeSize;
use crate::{Tile, TileExt, Window};
use std::task::Poll;

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl EventState {
    /// Construct per-window event state
    #[inline]
    pub(crate) fn new(window_id: WindowId, config: WindowConfig, platform: Platform) -> Self {
        EventState {
            window_id,
            config,
            platform,
            disabled: vec![],
            window_has_focus: false,
            #[cfg(feature = "accesskit")]
            accesskit_is_enabled: false,
            modifiers: ModifiersState::empty(),
            key_focus: false,
            ime: None,
            old_ime_target: None,
            ime_cursor_area: Rect::ZERO,
            last_ime_rect: Rect::ZERO,
            sel_focus: None,
            nav_focus: None,
            nav_fallback: None,
            key_depress: Default::default(),
            mouse: Default::default(),
            touch: Default::default(),
            access_layers: Default::default(),
            popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            frame_updates: Default::default(),
            need_frame_update: false,
            send_queue: Default::default(),
            fut_messages: vec![],
            pending_update: None,
            pending_sel_focus: None,
            pending_nav_focus: PendingNavFocus::None,
            action: Action::empty(),
        }
    }

    #[cfg(feature = "accesskit")]
    pub(crate) fn accesskit_tree_update<A>(&mut self, root: &Window<A>) -> accesskit::TreeUpdate {
        self.accesskit_is_enabled = true;

        let (nodes, root_id) = crate::accesskit::window_nodes(root);
        let tree = Some(accesskit::Tree::new(root_id));

        // AccessKit does not like focus to point at a non-existant node, so we
        // filter. See https://github.com/AccessKit/accesskit/issues/587
        let focus = self
            .nav_focus()
            .map(|id| id.into())
            .filter(|node_id| nodes.iter().any(|(id, _)| id == node_id))
            .unwrap_or(root_id);

        accesskit::TreeUpdate { nodes, tree, focus }
    }

    #[cfg(feature = "accesskit")]
    pub(crate) fn disable_accesskit(&mut self) {
        self.accesskit_is_enabled = false;
    }

    /// Update scale factor
    pub(crate) fn update_config(&mut self, scale_factor: f32) {
        self.config.update(scale_factor);
    }

    /// Configure a widget tree
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    ///
    /// This method calls [`ConfigCx::configure`] in order to assign
    /// [`Id`] identifiers and call widgets' [`Events::configure`]
    /// method. Additionally, it updates the [`EventState`] to account for
    /// renamed and removed widgets.
    pub(crate) fn full_configure<A>(
        &mut self,
        sizer: &dyn ThemeSize,
        win: &mut Window<A>,
        data: &A,
    ) {
        let id = Id::ROOT.make_child(self.window_id.get().cast());

        log::debug!(target: "kas_core::event", "full_configure of Window{id}");

        // These are recreated during configure:
        self.access_layers.clear();
        self.nav_fallback = None;

        self.new_access_layer(id.clone(), false);

        ConfigCx::new(sizer, self).configure(win.as_node(data), id);
        self.action |= Action::REGION_MOVED;
    }

    /// Get the next resume time
    pub(crate) fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    pub(crate) fn need_frame_update(&self) -> bool {
        self.need_frame_update || !self.frame_updates.is_empty() || !self.fut_messages.is_empty()
    }

    /// Construct a [`EventCx`] referring to this state
    ///
    /// Invokes the given closure on this [`EventCx`].
    #[inline]
    pub(crate) fn with<'a, F: FnOnce(&mut EventCx)>(
        &'a mut self,
        runner: &'a mut dyn RunnerT,
        window: &'a dyn WindowDataErased,
        messages: &'a mut MessageStack,
        f: F,
    ) {
        let mut cx = EventCx {
            state: self,
            runner,
            window,
            messages,
            target_is_disabled: false,
            last_child: None,
            scroll: Scroll::None,
        };
        f(&mut cx);
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

            match std::mem::take(&mut cx.pending_nav_focus) {
                PendingNavFocus::None => (),
                PendingNavFocus::Set { target, source } => {
                    cx.set_nav_focus_impl(win.as_node(data), target, source)
                }
                PendingNavFocus::Next {
                    target,
                    reverse,
                    source,
                } => cx.next_nav_focus_impl(win.as_node(data), target, reverse, source),
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

#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl<'a> EventCx<'a> {
    #[cfg(feature = "accesskit")]
    pub(crate) fn handle_accesskit_action(
        &mut self,
        widget: Node<'_>,
        request: accesskit::ActionRequest,
    ) {
        let Some(id) = Id::try_from_u64(request.target.0) else {
            return;
        };

        // TODO: implement remaining actions
        use crate::messages::{self, Erased, SetValueF64, SetValueText};
        use accesskit::{Action as AKA, ActionData};
        match request.action {
            AKA::Click => {
                self.send_event(widget, id, Event::Command(Command::Activate, None));
            }
            AKA::Focus => self.set_nav_focus(id, FocusSource::Synthetic),
            AKA::Blur => (),
            AKA::Collapse | AKA::Expand => (), // TODO: open/close menus
            AKA::CustomAction => (),
            AKA::Decrement => {
                self.send_or_replay(widget, id, Erased::new(messages::DecrementStep));
            }
            AKA::Increment => {
                self.send_or_replay(widget, id, Erased::new(messages::IncrementStep));
            }
            AKA::HideTooltip | AKA::ShowTooltip => (),
            AKA::ReplaceSelectedText => (),
            AKA::ScrollDown | AKA::ScrollLeft | AKA::ScrollRight | AKA::ScrollUp => {
                let delta = match request.action {
                    AKA::ScrollDown => ScrollDelta::Lines(0.0, 1.0),
                    AKA::ScrollLeft => ScrollDelta::Lines(-1.0, 0.0),
                    AKA::ScrollRight => ScrollDelta::Lines(1.0, 0.0),
                    AKA::ScrollUp => ScrollDelta::Lines(0.0, -1.0),
                    _ => unreachable!(),
                };
                self.send_event(widget, id, Event::Scroll(delta));
            }
            AKA::ScrollIntoView | AKA::ScrollToPoint => {
                // We assume input is in coordinate system of target
                let scroll = match request.data {
                    None => {
                        debug_assert_eq!(request.action, AKA::ScrollIntoView);
                        // NOTE: we shouldn't need two tree traversals, but it's fine
                        if let Some(tile) = widget.as_tile().find_tile(&id) {
                            Scroll::Rect(tile.rect())
                        } else {
                            return;
                        }
                    }
                    Some(ActionData::ScrollToPoint(point)) => {
                        debug_assert_eq!(request.action, AKA::ScrollToPoint);
                        let pos = point.cast_approx();
                        let size = Size::ZERO;
                        Scroll::Rect(Rect { pos, size })
                    }
                    _ => {
                        debug_assert!(false);
                        return;
                    }
                };
                self.replay_scroll(widget, id, scroll);
            }
            AKA::SetScrollOffset => {
                if let Some(ActionData::SetScrollOffset(point)) = request.data {
                    let msg = kas::messages::SetScrollOffset(point.cast_approx());
                    self.send_or_replay(widget, id, Erased::new(msg));
                }
            }
            AKA::SetTextSelection => (),
            AKA::SetSequentialFocusNavigationStartingPoint => (),
            AKA::SetValue => {
                let msg = match request.data {
                    Some(ActionData::Value(text)) => Erased::new(SetValueText(text.into())),
                    Some(ActionData::NumericValue(n)) => Erased::new(SetValueF64(n)),
                    _ => return,
                };
                self.send_or_replay(widget, id, msg);
            }
            AKA::ShowContextMenu => (),
        }
    }

    /// Pre-draw / pre-sleep
    ///
    /// This method should be called once per frame as well as after the last
    /// frame before a long sleep.
    pub(crate) fn frame_update(&mut self, mut widget: Node<'_>) {
        self.need_frame_update = false;
        log::trace!(target: "kas_core::event", "Processing frame update");
        if let Some((target, affine)) = self.mouse.frame_update() {
            self.send_event(widget.re(), target, Event::Pan(affine));
        }
        self.touch_frame_update(widget.re());

        let frame_updates = std::mem::take(&mut self.frame_updates);
        for (id, handle) in frame_updates.into_iter() {
            self.send_event(widget.re(), id, Event::Timer(handle));
        }

        // Set IME cursor area, if moved.
        if self.ime.is_some()
            && let Some(target) = self.sel_focus.as_ref()
            && let Some((mut rect, translation)) = widget.as_tile().find_tile_rect(target)
        {
            if self.ime_cursor_area.size != Size::ZERO {
                rect = self.ime_cursor_area;
            }
            rect += translation;
            if rect != self.last_ime_rect {
                self.window.set_ime_cursor_area(rect);
                self.last_ime_rect = rect;
            }
        }
    }

    /// Update widgets due to timer
    pub(crate) fn update_timer(&mut self, mut widget: Node<'_>) {
        let now = Instant::now();

        // assumption: time_updates are sorted in reverse order
        while !self.time_updates.is_empty() {
            if self.time_updates.last().unwrap().0 > now {
                break;
            }

            let update = self.time_updates.pop().unwrap();
            self.send_event(widget.re(), update.1, Event::Timer(update.2));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    fn poll_futures(&mut self, mut widget: Node<'_>) {
        let mut i = 0;
        while i < self.state.fut_messages.len() {
            let (_, fut) = &mut self.state.fut_messages[i];
            let mut cx = std::task::Context::from_waker(self.runner.waker());
            match fut.as_mut().poll(&mut cx) {
                Poll::Pending => {
                    i += 1;
                }
                Poll::Ready(msg) => {
                    let (id, _) = self.state.fut_messages.remove(i);

                    // Replay message. This could push another future; if it
                    // does we should poll it immediately to start its work.
                    self.send_or_replay(widget.re(), id, msg);
                }
            }
        }
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
                    self.action(win.id(), Action::REDRAW);
                } else {
                    // Window focus lost: close all popups
                    while let Some(id) = self.popups.last().map(|state| state.id) {
                        self.close_window(id);
                    }
                }
            }
            KeyboardInput {
                mut event,
                is_synthetic,
                ..
            } => {
                let state = event.state;
                let physical_key = event.physical_key;
                let logical_key = event.logical_key.clone();

                if let Some(id) = self.key_focus() {
                    // TODO(winit): https://github.com/rust-windowing/winit/issues/3038
                    let mut mods = self.modifiers;
                    mods.remove(ModifiersState::SHIFT);
                    if !mods.is_empty()
                        || event
                            .text
                            .as_ref()
                            .and_then(|t| t.chars().next())
                            .map(|c| c.is_control())
                            .unwrap_or(false)
                    {
                        event.text = None;
                    }

                    if self.send_event(win.as_node(data), id, Event::Key(&event, is_synthetic)) {
                        return;
                    }
                }

                if state == ElementState::Pressed && !is_synthetic {
                    self.start_key_event(win.as_node(data), logical_key, physical_key);
                } else if state == ElementState::Released
                    && let Some(id) = self.key_depress.remove(&physical_key)
                {
                    self.redraw(id);
                }
            }
            ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                if state.alt_key() != self.modifiers.alt_key() {
                    // This controls drawing of access key indicators
                    self.action(win.id(), Action::REDRAW);
                }
                self.modifiers = state;
            }
            Ime(winit::event::Ime::Enabled) => {
                // We expect self.ime.is_some(), but it's possible that the request is outdated
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(win.as_node(data), id, Event::ImeFocus);
                }
            }
            Ime(winit::event::Ime::Disabled) => {
                // We can only assume that this is received due to us disabling
                // IME if self.old_ime_target is set, and is otherwise due to an
                // external cause.
                let mut target = self.old_ime_target.take();
                if target.is_none() && self.ime.is_some() {
                    target = self.sel_focus.clone();
                    self.ime = None;
                    self.ime_cursor_area = Rect::ZERO;
                }
                if let Some(id) = target {
                    self.send_event(win.as_node(data), id, Event::LostImeFocus);
                }
            }
            Ime(winit::event::Ime::Preedit(text, cursor)) => {
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(win.as_node(data), id, Event::ImePreedit(&text, cursor));
                }
            }
            Ime(winit::event::Ime::Commit(text)) => {
                if self.ime.is_some()
                    && let Some(id) = self.sel_focus.clone()
                {
                    self.send_event(win.as_node(data), id, Event::ImeCommit(&text));
                }
            }
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
