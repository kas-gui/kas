// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — platform API

use std::task::Poll;
use std::time::Duration;

use super::*;
use crate::cast::traits::*;
use crate::geom::DVec2;
use crate::theme::ThemeSize;
use crate::Window;

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

/// Platform API
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
            modifiers: ModifiersState::empty(),
            key_focus: false,
            sel_focus: None,
            nav_focus: None,
            nav_fallback: None,
            hover: None,
            hover_icon: CursorIcon::Default,
            old_hover_icon: CursorIcon::Default,
            key_depress: Default::default(),
            last_mouse_coord: Coord::ZERO,
            last_click_button: FAKE_MOUSE_BUTTON,
            last_click_repetitions: 0,
            last_click_timeout: Instant::now(), // unimportant value
            mouse_grab: None,
            touch_grab: Default::default(),
            pan_grab: SmallVec::new(),
            access_layers: Default::default(),
            popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            send_queue: Default::default(),
            fut_messages: vec![],
            pending_update: None,
            pending_sel_focus: None,
            pending_nav_focus: PendingNavFocus::None,
            pending_cmds: Default::default(),
            action: Action::empty(),
        }
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
        self.action.remove(Action::RECONFIGURE);

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

            let mut cancel = false;
            if let Some(grab) = cx.state.mouse_grab.as_mut() {
                cancel = grab.cancel;
                if let GrabDetails::Click = grab.details {
                    let hover = cx.state.hover.as_ref();
                    if grab.start_id == hover {
                        if grab.depress.as_ref() != hover {
                            grab.depress = hover.cloned();
                            cx.action |= Action::REDRAW;
                        }
                    } else if grab.depress.is_some() {
                        grab.depress = None;
                        cx.action |= Action::REDRAW;
                    }
                }
            }
            if cancel {
                if let Some((id, event)) = cx.remove_mouse_grab(false) {
                    cx.send_event(win.as_node(data), id, event);
                }
            }

            let mut i = 0;
            while i < cx.touch_grab.len() {
                let action = cx.touch_grab[i].flush_click_move();
                cx.state.action |= action;

                if cx.touch_grab[i].cancel {
                    let grab = cx.remove_touch(i);

                    let press = Press {
                        source: PressSource::Touch(grab.id),
                        id: grab.cur_id,
                        coord: grab.coord,
                    };
                    let event = Event::PressEnd {
                        press,
                        success: false,
                    };
                    cx.send_event(win.as_node(data), grab.start_id, event);
                } else {
                    i += 1;
                }
            }

            for gi in 0..cx.pan_grab.len() {
                let grab = &mut cx.pan_grab[gi];
                debug_assert!(grab.mode != GrabMode::Grab);
                assert!(grab.n > 0);

                // Terminology: pi are old coordinates, qi are new coords
                let (p1, q1) = (DVec2::conv(grab.coords[0].0), DVec2::conv(grab.coords[0].1));
                grab.coords[0].0 = grab.coords[0].1;

                let alpha;
                let delta;

                if grab.mode == GrabMode::PanOnly || grab.n == 1 {
                    alpha = DVec2(1.0, 0.0);
                    delta = q1 - p1;
                } else {
                    // We don't use more than two touches: information would be
                    // redundant (although it could be averaged).
                    let (p2, q2) = (DVec2::conv(grab.coords[1].0), DVec2::conv(grab.coords[1].1));
                    grab.coords[1].0 = grab.coords[1].1;
                    let (pd, qd) = (p2 - p1, q2 - q1);

                    alpha = match grab.mode {
                        GrabMode::PanFull => qd.complex_div(pd),
                        GrabMode::PanScale => {
                            DVec2((qd.sum_square() / pd.sum_square()).sqrt(), 0.0)
                        }
                        GrabMode::PanRotate => {
                            let a = qd.complex_div(pd);
                            a / a.sum_square().sqrt()
                        }
                        _ => unreachable!(),
                    };

                    // Average delta from both movements:
                    delta = (q1 - alpha.complex_mul(p1) + q2 - alpha.complex_mul(p2)) * 0.5;
                }

                let id = grab.id.clone();
                if alpha != DVec2(1.0, 0.0) || delta != DVec2::ZERO {
                    let event = Event::Pan { alpha, delta };
                    cx.send_event(win.as_node(data), id, event);
                }
            }

            if let Some((id, reconf)) = cx.pending_update.take() {
                if reconf {
                    win.as_node(data)
                        .find_node(&id, |node| cx.configure(node, id.clone()));

                    cx.action |= Action::REGION_MOVED;
                } else {
                    win.as_node(data).find_node(&id, |node| cx.update(node));
                }
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
                cx.set_sel_focus(win.as_node(data), pending);
            }

            while let Some((id, cmd)) = cx.pending_cmds.pop_front() {
                if cmd == Command::Exit {
                    cx.runner.exit();
                } else if cmd == Command::Close {
                    cx.handle_close();
                } else {
                    log::trace!(target: "kas_core::event", "sending pending command {cmd:?} to {id}");
                    cx.send_event(win.as_node(data), id, Event::Command(cmd, None));
                }
            }

            while let Some((id, msg)) = cx.send_queue.pop_front() {
                log::trace!(target: "kas_core::event", "sending message {msg:?} to {id}");
                cx.replay(win.as_node(data), id, msg);
            }

            // Poll futures almost last. This means that any newly pushed future
            // should get polled from the same update() call.
            cx.poll_futures(win.as_node(data));

            // Finally, clear the region_moved flag.
            if cx.action.contains(Action::REGION_MOVED) {
                cx.action.remove(Action::REGION_MOVED);

                // Update hovered widget
                let hover = win.try_probe(cx.last_mouse_coord);
                cx.set_hover(win.as_node(data), hover);

                for grab in cx.touch_grab.iter_mut() {
                    grab.cur_id = win.try_probe(grab.coord);
                }
            }
        });

        if self.hover_icon != self.old_hover_icon && self.mouse_grab.is_none() {
            window.set_cursor_icon(self.hover_icon);
        }
        self.old_hover_icon = self.hover_icon;

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

/// Platform API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(docsrs, doc(cfg(internal_doc)))]
impl<'a> EventCx<'a> {
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
                    self.replay(widget.re(), id, msg);
                }
            }
        }
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not handled, since for these
    /// events the graphics backend must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(winit)]
    pub(crate) fn handle_winit<A>(
        &mut self,
        win: &mut Window<A>,
        data: &A,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{MouseScrollDelta, TouchPhase, WindowEvent::*};

        match event {
            CloseRequested => self.action(Id::ROOT, Action::CLOSE),
            /* Not yet supported: see #98
            DroppedFile(path) => ,
            HoveredFile(path) => ,
            HoveredFileCancelled => ,
            */
            Focused(state) => {
                self.window_has_focus = state;
                if state {
                    // Required to restart theme animations
                    self.action(Id::ROOT, Action::REDRAW);
                } else {
                    // Window focus lost: close all popups
                    while let Some(id) = self.popups.last().map(|(id, _, _)| *id) {
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

                    if self.send_event(win.as_node(data), id, Event::Key(event, is_synthetic)) {
                        return;
                    }
                }

                if state == ElementState::Pressed && !is_synthetic {
                    self.start_key_event(win.as_node(data), logical_key, physical_key);
                } else if state == ElementState::Released {
                    if let Some(id) = self.key_depress.remove(&physical_key) {
                        self.redraw(id);
                    }
                }
            }
            ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                if state.alt_key() != self.modifiers.alt_key() {
                    // This controls drawing of access key indicators
                    self.action(Id::ROOT, Action::REDRAW);
                }
                self.modifiers = state;
            }
            CursorMoved { position, .. } => {
                self.last_click_button = FAKE_MOUSE_BUTTON;
                let coord = position.cast_approx();

                // Update hovered win
                let id = win.try_probe(coord);
                self.set_hover(win.as_node(data), id.clone());

                if let Some(grab) = self.state.mouse_grab.as_mut() {
                    match grab.details {
                        GrabDetails::Click => (),
                        GrabDetails::Grab => {
                            let target = grab.start_id.clone();
                            let press = Press {
                                source: PressSource::Mouse(grab.button, grab.repetitions),
                                id,
                                coord,
                            };
                            let delta = coord - self.last_mouse_coord;
                            let event = Event::PressMove { press, delta };
                            self.send_event(win.as_node(data), target, event);
                        }
                        GrabDetails::Pan(g) => {
                            if let Some(pan) = self.state.pan_grab.get_mut(usize::conv(g.0)) {
                                pan.coords[usize::conv(g.1)].1 = coord;
                            }
                        }
                    }
                } else if let Some(popup_id) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
                    let press = Press {
                        source: PressSource::Mouse(FAKE_MOUSE_BUTTON, 0),
                        id,
                        coord,
                    };
                    let event = Event::CursorMove { press };
                    self.send_event(win.as_node(data), popup_id, event);
                } else {
                    // We don't forward move events without a grab
                }

                self.last_mouse_coord = coord;
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                self.last_click_button = FAKE_MOUSE_BUTTON;

                if self.mouse_grab.is_none() {
                    // If there's a mouse grab, we will continue to receive
                    // coordinates; if not, set a fake coordinate off the window
                    self.last_mouse_coord = Coord(-1, -1);
                    self.set_hover(win.as_node(data), None);
                }
            }
            MouseWheel { delta, .. } => {
                self.last_click_button = FAKE_MOUSE_BUTTON;

                let event = Event::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) => {
                        // The delta is given as a PhysicalPosition, so we need
                        // to convert to our vector type (Offset) here.
                        let coord = Coord::conv_approx(pos);
                        ScrollDelta::PixelDelta(coord.cast())
                    }
                });
                if let Some(id) = self.hover.clone() {
                    self.send_event(win.as_node(data), id, event);
                }
            }
            MouseInput { state, button, .. } => {
                let coord = self.last_mouse_coord;

                if state == ElementState::Pressed {
                    let now = Instant::now();
                    if button != self.last_click_button || self.last_click_timeout < now {
                        self.last_click_button = button;
                        self.last_click_repetitions = 0;
                    }
                    self.last_click_repetitions += 1;
                    self.last_click_timeout = now + DOUBLE_CLICK_TIMEOUT;
                }

                if self
                    .mouse_grab
                    .as_ref()
                    .map(|g| g.button == button)
                    .unwrap_or(false)
                {
                    if let Some((id, event)) = self.remove_mouse_grab(true) {
                        self.send_event(win.as_node(data), id, event);
                    }
                }

                if state == ElementState::Pressed {
                    if let Some(start_id) = self.hover.clone() {
                        // No mouse grab but have a hover target
                        if self.config.event().mouse_nav_focus() {
                            if let Some(id) =
                                self.nav_next(win.as_node(data), Some(&start_id), NavAdvance::None)
                            {
                                self.set_nav_focus(id, FocusSource::Pointer);
                            }
                        }
                    }

                    let source = PressSource::Mouse(button, self.last_click_repetitions);
                    let press = Press {
                        source,
                        id: self.hover.clone(),
                        coord,
                    };
                    let event = Event::PressStart { press };
                    self.send_popup_first(win.as_node(data), self.hover.clone(), event);
                }
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            Touch(touch) => {
                let source = PressSource::Touch(touch.id);
                let coord = touch.location.cast_approx();
                match touch.phase {
                    TouchPhase::Started => {
                        let start_id = win.try_probe(coord);
                        if let Some(id) = start_id.as_ref() {
                            if self.config.event().touch_nav_focus() {
                                if let Some(id) =
                                    self.nav_next(win.as_node(data), Some(id), NavAdvance::None)
                                {
                                    self.set_nav_focus(id, FocusSource::Pointer);
                                }
                            }

                            let press = Press {
                                source,
                                id: start_id.clone(),
                                coord,
                            };
                            let event = Event::PressStart { press };
                            self.send_popup_first(win.as_node(data), start_id, event);
                        }
                    }
                    TouchPhase::Moved => {
                        let cur_id = win.try_probe(coord);

                        let mut redraw = false;
                        let mut pan_grab = None;
                        if let Some(grab) = self.get_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                // Only when 'depressed' status changes:
                                redraw = grab.cur_id != cur_id
                                    && (grab.start_id == grab.cur_id || grab.start_id == cur_id);

                                grab.cur_id = cur_id;
                                grab.coord = coord;

                                if grab.last_move != grab.coord {
                                    let delta = grab.coord - grab.last_move;
                                    let target = grab.start_id.clone();
                                    let press = Press {
                                        source: PressSource::Touch(grab.id),
                                        id: grab.cur_id.clone(),
                                        coord: grab.coord,
                                    };
                                    let event = Event::PressMove { press, delta };
                                    grab.last_move = grab.coord;
                                    self.send_event(win.as_node(data), target, event);
                                }
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if redraw {
                            self.action(Id::ROOT, Action::REDRAW);
                        } else if let Some(pan_grab) = pan_grab {
                            if usize::conv(pan_grab.1) < MAX_PAN_GRABS {
                                if let Some(pan) = self.pan_grab.get_mut(usize::conv(pan_grab.0)) {
                                    pan.coords[usize::conv(pan_grab.1)].1 = coord;
                                }
                            }
                        }
                    }
                    ev @ (TouchPhase::Ended | TouchPhase::Cancelled) => {
                        if let Some(index) = self.get_touch_index(touch.id) {
                            let grab = self.remove_touch(index);

                            if grab.mode == GrabMode::Grab {
                                let id = grab.cur_id.clone();
                                let press = Press { source, id, coord };
                                let success = ev == TouchPhase::Ended;
                                let event = Event::PressEnd { press, success };
                                self.send_event(win.as_node(data), grab.start_id, event);
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
