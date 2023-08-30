// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — shell API

use smallvec::SmallVec;
use std::task::Poll;
use std::time::{Duration, Instant};

use super::*;
use crate::cast::traits::*;
use crate::draw::DrawShared;
use crate::geom::{Coord, DVec2};
use crate::shell::ShellWindow;
use crate::theme::ThemeSize;
use crate::{Action, NavAdvance, WidgetId, Window};

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl EventState {
    /// Construct per-window event state
    #[inline]
    pub(crate) fn new(config: WindowConfig) -> Self {
        EventState {
            config,
            disabled: vec![],
            window_has_focus: false,
            modifiers: ModifiersState::empty(),
            key_focus: false,
            sel_focus: None,
            nav_focus: None,
            nav_fallback: None,
            hover: None,
            hover_icon: CursorIcon::Default,
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
            fut_messages: vec![],
            pending: Default::default(),
            action: Action::empty(),
        }
    }

    /// Update scale factor
    pub(crate) fn update_config(&mut self, scale_factor: f32, dpem: f32) {
        self.config.update(scale_factor, dpem);
    }

    /// Configure a widget tree
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    ///
    /// This method calls [`ConfigCx::configure`] in order to assign
    /// [`WidgetId`] identifiers and call widgets' [`Events::configure`]
    /// method. Additionally, it updates the [`EventState`] to account for
    /// renamed and removed widgets.
    pub(crate) fn full_configure<A>(
        &mut self,
        sizer: &dyn ThemeSize,
        draw_shared: &mut dyn DrawShared,
        wid: WindowId,
        win: &mut Window<A>,
        data: &A,
    ) {
        let id = WidgetId::ROOT.make_child(wid.get().cast());

        log::debug!(target: "kas_core::event", "full_configure of Window{id}");
        self.action.remove(Action::RECONFIGURE);

        // These are recreated during configure:
        self.access_layers.clear();
        self.nav_fallback = None;

        self.new_access_layer(id.clone(), false);

        ConfigCx::new(sizer, draw_shared, self).configure(win.as_node(data), id);

        let hover = win.find_id(data, self.last_mouse_coord);
        self.set_hover(hover);
    }

    /// Update the widgets under the cursor and touch events
    pub(crate) fn region_moved<A>(&mut self, win: &mut Window<A>, data: &A) {
        log::trace!(target: "kas_core::event", "region_moved");
        // Note: redraw is already implied.

        // Update hovered widget
        let hover = win.find_id(data, self.last_mouse_coord);
        self.set_hover(hover);

        for grab in self.touch_grab.iter_mut() {
            grab.cur_id = win.find_id(data, grab.coord);
        }
    }

    /// Get the next resume time
    pub(crate) fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    /// Construct a [`EventCx`] referring to this state
    ///
    /// Invokes the given closure on this [`EventCx`].
    #[inline]
    pub(crate) fn with<F>(&mut self, shell: &mut dyn ShellWindow, messages: &mut ErasedStack, f: F)
    where
        F: FnOnce(&mut EventCx),
    {
        let mut cx = EventCx {
            state: self,
            shell,
            messages,
            last_child: None,
            scroll: Scroll::None,
        };
        f(&mut cx);
    }

    /// Handle all pending items before event loop sleeps
    pub(crate) fn flush_pending<A>(
        &mut self,
        shell: &mut dyn ShellWindow,
        messages: &mut ErasedStack,
        win: &mut Window<A>,
        data: &A,
    ) -> Action {
        let old_hover_icon = self.hover_icon;

        let mut cx = EventCx {
            state: self,
            shell,
            messages,
            last_child: None,
            scroll: Scroll::None,
        };

        while let Some((id, wid)) = cx.popup_removed.pop() {
            cx.send_event(win.as_node(data), id, Event::PopupClosed(wid));
        }

        cx.flush_mouse_grab_motion(win.as_node(data));
        for i in 0..cx.touch_grab.len() {
            let action = cx.touch_grab[i].flush_click_move();
            cx.state.action |= action;
            if let Some((id, event)) = cx.touch_grab[i].flush_grab_move() {
                cx.send_event(win.as_node(data), id, event);
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
                    GrabMode::PanScale => DVec2((qd.sum_square() / pd.sum_square()).sqrt(), 0.0),
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

        // Warning: infinite loops are possible here if widgets always queue a
        // new pending event when evaluating one of these:
        while let Some(item) = cx.pending.pop_front() {
            log::trace!(target: "kas_core::event", "update: handling Pending::{item:?}");
            match item {
                Pending::Configure(id) => {
                    win.as_node(data)
                        .find_node(&id, |node| cx.configure(node, id.clone()));

                    let hover = win.find_id(data, cx.state.last_mouse_coord);
                    cx.state.set_hover(hover);
                }
                Pending::Update(id) => {
                    win.as_node(data).find_node(&id, |node| cx.update(node));
                }
                Pending::Send(id, event) => {
                    if matches!(&event, &Event::MouseHover(false)) {
                        cx.hover_icon = Default::default();
                    }
                    cx.send_event(win.as_node(data), id, event);
                }
                Pending::SetRect(_id) => {
                    // TODO(opt): set only this child
                    cx.send_action(Action::SET_RECT);
                }
                Pending::NextNavFocus {
                    target,
                    reverse,
                    source,
                } => {
                    cx.next_nav_focus_impl(win.as_node(data), target, reverse, source);
                }
            }
        }

        // Poll futures last. This means that any newly pushed future should
        // get polled from the same update() call.
        cx.poll_futures(win.as_node(data));

        drop(cx);

        if self.hover_icon != old_hover_icon && self.mouse_grab.is_none() {
            shell.set_cursor_icon(self.hover_icon);
        }

        std::mem::take(&mut self.action)
    }
}

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
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
            self.send_event(widget.re(), update.1, Event::TimerUpdate(update.2));
        }

        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    fn poll_futures(&mut self, mut widget: Node<'_>) {
        let mut i = 0;
        while i < self.state.fut_messages.len() {
            let (_, fut) = &mut self.state.fut_messages[i];
            let mut cx = std::task::Context::from_waker(self.shell.waker());
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
    /// events the shell must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(winit)]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
    pub(crate) fn handle_winit<A>(
        &mut self,
        data: &A,
        win: &mut Window<A>,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{MouseScrollDelta, TouchPhase, WindowEvent::*};

        match event {
            CloseRequested => self.send_action(Action::CLOSE),
            /* Not yet supported: see #98
            DroppedFile(path) => ,
            HoveredFile(path) => ,
            HoveredFileCancelled => ,
            */
            Focused(state) => {
                self.window_has_focus = state;
                if state {
                    // Required to restart theme animations
                    self.send_action(Action::REDRAW);
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
                    if !mods.is_empty() {
                        event.text = None;
                    } else if event
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
                    self.send_action(Action::REDRAW);
                }
                self.modifiers = state;
            }
            CursorMoved { position, .. } => {
                self.last_click_button = FAKE_MOUSE_BUTTON;
                let coord = position.cast_approx();

                // Update hovered win
                let cur_id = win.find_id(data, coord);
                let delta = coord - self.last_mouse_coord;
                self.set_hover(cur_id.clone());

                if let Some(grab) = self.state.mouse_grab.as_mut() {
                    if !grab.mode.is_pan() {
                        grab.cur_id = cur_id;
                        grab.coord = coord;
                        grab.delta += delta;
                    } else if let Some(pan) =
                        self.state.pan_grab.get_mut(usize::conv(grab.pan_grab.0))
                    {
                        pan.coords[usize::conv(grab.pan_grab.1)].1 = coord;
                    }
                } else if let Some(id) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
                    let press = Press {
                        source: PressSource::Mouse(FAKE_MOUSE_BUTTON, 0),
                        id: cur_id,
                        coord,
                    };
                    let event = Event::CursorMove { press };
                    self.send_event(win.as_node(data), id, event);
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
                    self.set_hover(None);
                }
            }
            MouseWheel { delta, .. } => {
                self.flush_mouse_grab_motion(win.as_node(data));

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
                self.flush_mouse_grab_motion(win.as_node(data));

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
                        if self.config.mouse_nav_focus() {
                            if let Some(id) =
                                win._nav_next(self, data, Some(&start_id), NavAdvance::None)
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
                        let start_id = win.find_id(data, coord);
                        if let Some(id) = start_id.as_ref() {
                            if self.config.touch_nav_focus() {
                                if let Some(id) =
                                    win._nav_next(self, data, Some(id), NavAdvance::None)
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
                        let cur_id = win.find_id(data, coord);

                        let mut redraw = false;
                        let mut pan_grab = None;
                        if let Some(grab) = self.get_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                // Only when 'depressed' status changes:
                                redraw = grab.cur_id != cur_id
                                    && (grab.start_id == grab.cur_id || grab.start_id == cur_id);

                                grab.cur_id = cur_id;
                                grab.coord = coord;
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if redraw {
                            self.send_action(Action::REDRAW);
                        } else if let Some(pan_grab) = pan_grab {
                            if usize::conv(pan_grab.1) < MAX_PAN_GRABS {
                                if let Some(pan) = self.pan_grab.get_mut(usize::conv(pan_grab.0)) {
                                    pan.coords[usize::conv(pan_grab.1)].1 = coord;
                                }
                            }
                        }
                    }
                    ev @ (TouchPhase::Ended | TouchPhase::Cancelled) => {
                        if let Some(mut grab) = self.remove_touch(touch.id) {
                            self.send_action(grab.flush_click_move());
                            if let Some((id, event)) = grab.flush_grab_move() {
                                self.send_event(win.as_node(data), id, event);
                            }

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
