// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — shell API

use log::*;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::*;
use crate::cast::traits::*;
use crate::geom::{Coord, DVec2};
use crate::layout::SetRectMgr;
use crate::{ShellWindow, TkAction, Widget, WidgetId};

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl EventState {
    /// Construct an event manager per-window data struct
    #[inline]
    pub fn new(config: Rc<RefCell<Config>>, scale_factor: f32) -> Self {
        EventState {
            config: WindowConfig::new(config, scale_factor),
            disabled: vec![],
            window_has_focus: false,
            modifiers: ModifiersState::empty(),
            char_focus: false,
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
            accel_layers: Default::default(),
            popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            handle_updates: HashMap::new(),
            pending: SmallVec::new(),
            action: TkAction::empty(),
        }
    }

    /// Update scale factor
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.config.set_scale_factor(scale_factor);
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    ///
    /// This method calls [`SetRectMgr::configure`] in order to assign
    /// [`WidgetId`] identifiers and call widgets' [`Widget::configure`]
    /// method. Additionally, it updates the [`EventState`] to account for
    /// renamed and removed widgets.
    pub fn full_configure(&mut self, shell: &mut dyn ShellWindow, widget: &mut dyn Widget) {
        debug!("EventMgr::configure");
        self.action.remove(TkAction::RECONFIGURE);

        // These are recreated during configure:
        self.accel_layers.clear();
        self.nav_fallback = None;

        self.new_accel_layer(WidgetId::ROOT, false);

        shell.size_and_draw_shared(&mut |size_handle, draw_shared| {
            let mut mgr = SetRectMgr::new(size_handle, draw_shared, self);
            mgr.configure(WidgetId::ROOT, widget);
        });

        let hover = widget.find_id(self.last_mouse_coord);
        self.with(shell, |mgr| mgr.set_hover(widget, hover));
    }

    /// Update the widgets under the cursor and touch events
    pub fn region_moved(&mut self, shell: &mut dyn ShellWindow, widget: &mut dyn Widget) {
        trace!("EventMgr::region_moved");
        // Note: redraw is already implied.

        // Update hovered widget
        let hover = widget.find_id(self.last_mouse_coord);
        self.with(shell, |mgr| mgr.set_hover(widget, hover));

        for grab in self.touch_grab.iter_mut() {
            grab.cur_id = widget.find_id(grab.coord);
        }
    }

    /// Get the next resume time
    pub fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    /// Construct a [`EventMgr`] referring to this state
    ///
    /// Invokes the given closure on this [`EventMgr`].
    #[inline]
    pub fn with<F>(&mut self, shell: &mut dyn ShellWindow, f: F)
    where
        F: FnOnce(&mut EventMgr),
    {
        let mut mgr = EventMgr {
            state: self,
            shell,
            messages: vec![],
            scroll: Scroll::None,
            action: TkAction::empty(),
        };
        f(&mut mgr);
        let action = mgr.action;
        drop(mgr);
        self.send_action(action);
    }

    /// Update, after receiving all events
    #[inline]
    pub fn update(&mut self, shell: &mut dyn ShellWindow, widget: &mut dyn Widget) -> TkAction {
        let mut mgr = EventMgr {
            state: self,
            shell,
            messages: vec![],
            scroll: Scroll::None,
            action: TkAction::empty(),
        };

        while let Some((parent, wid)) = mgr.state.popup_removed.pop() {
            mgr.send_event(widget, parent, Event::PopupRemoved(wid));
        }

        if let Some((id, event)) = mgr.mouse_grab().and_then(|g| g.flush_move()) {
            mgr.send_event(widget, id, event);
        }

        for i in 0..mgr.state.touch_grab.len() {
            if let Some((id, event)) = mgr.state.touch_grab[i].flush_move() {
                mgr.send_event(widget, id, event);
            }
        }

        for gi in 0..mgr.state.pan_grab.len() {
            let grab = &mut mgr.state.pan_grab[gi];
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
                mgr.send_event(widget, id, event);
            }
        }

        // Warning: infinite loops are possible here if widgets always queue a
        // new pending event when evaluating one of these:
        while let Some(item) = mgr.state.pending.pop() {
            trace!("Handling Pending::{:?}", item);
            let (id, event) = match item {
                Pending::LostCharFocus(id) => (id, Event::LostCharFocus),
                Pending::LostSelFocus(id) => (id, Event::LostSelFocus),
                Pending::SetNavFocus(id, key_focus) => (id, Event::NavFocus(key_focus)),
            };
            mgr.send_event(widget, id, event);
        }

        let action = mgr.action;
        drop(mgr);
        let action = action | self.action;
        self.action = TkAction::empty();
        action
    }
}

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<'a> EventMgr<'a> {
    /// Update widgets due to timer
    pub fn update_timer(&mut self, widget: &mut dyn Widget) {
        let now = Instant::now();

        // assumption: time_updates are sorted in reverse order
        while !self.state.time_updates.is_empty() {
            if self.state.time_updates.last().unwrap().0 > now {
                break;
            }

            let update = self.state.time_updates.pop().unwrap();
            self.send_event(widget, update.1, Event::TimerUpdate(update.2));
        }

        self.state.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort
    }

    /// Update widgets due to handle
    pub fn update_handle(&mut self, widget: &mut dyn Widget, handle: UpdateHandle, payload: u64) {
        // NOTE: to avoid borrow conflict, we must clone values!
        if let Some(mut values) = self.state.handle_updates.get(&handle).cloned() {
            for w_id in values.drain() {
                let event = Event::HandleUpdate { handle, payload };
                self.send_event(widget, w_id, event);
            }
        }
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not handled, since for these
    /// events the shell must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    #[cfg_attr(doc_cfg, doc(cfg(feature = "winit")))]
    pub fn handle_winit(&mut self, widget: &mut dyn Widget, event: winit::event::WindowEvent) {
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};

        match event {
            CloseRequested => self.send_action(TkAction::CLOSE),
            /* Not yet supported: see #98
            DroppedFile(path) => ,
            HoveredFile(path) => ,
            HoveredFileCancelled => ,
            */
            ReceivedCharacter(c) => {
                if let Some(id) = self.state.char_focus() {
                    // Filter out control codes (Unicode 5.11). These may be
                    // generated from combinations such as Ctrl+C by some other
                    // layer. We use our own shortcut system instead.
                    if c >= '\x20' && !('\x7f'..='\u{9f}').contains(&c) {
                        let event = Event::ReceivedCharacter(c);
                        self.send_event(widget, id, event);
                    }
                }
            }
            Focused(state) => {
                self.state.window_has_focus = state;
                if state {
                    // Required to restart theme animations
                    self.state.send_action(TkAction::REDRAW);
                } else {
                    // Window focus lost: close all popups
                    while let Some(id) = self.state.popups.last().map(|(id, _, _)| *id) {
                        self.close_window(id, true);
                    }
                }
            }
            KeyboardInput {
                input,
                is_synthetic,
                ..
            } => {
                if input.state == ElementState::Pressed && !is_synthetic {
                    if let Some(vkey) = input.virtual_keycode {
                        self.start_key_event(widget, vkey, input.scancode);
                    }
                } else if input.state == ElementState::Released {
                    self.end_key_event(input.scancode);
                }
            }
            ModifiersChanged(state) => {
                if state.alt() != self.state.modifiers.alt() {
                    // This controls drawing of accelerator key indicators
                    self.state.send_action(TkAction::REDRAW);
                }
                self.state.modifiers = state;
            }
            CursorMoved { position, .. } => {
                self.state.last_click_button = FAKE_MOUSE_BUTTON;
                let coord = position.cast_approx();

                // Update hovered widget
                let cur_id = widget.find_id(coord);
                let delta = coord - self.state.last_mouse_coord;
                self.set_hover(widget, cur_id.clone());

                if let Some(grab) = self.state.mouse_grab.as_mut() {
                    if grab.mode == GrabMode::Grab {
                        grab.cur_id = cur_id;
                        grab.coord = coord;
                        grab.delta += delta;
                    } else if let Some(pan) =
                        self.state.pan_grab.get_mut(usize::conv(grab.pan_grab.0))
                    {
                        pan.coords[usize::conv(grab.pan_grab.1)].1 = coord;
                    }
                } else if let Some(id) = self.state.popups.last().map(|(_, p, _)| p.parent.clone())
                {
                    let source = PressSource::Mouse(FAKE_MOUSE_BUTTON, 0);
                    let event = Event::PressMove {
                        source,
                        cur_id,
                        coord,
                        delta,
                    };
                    self.send_event(widget, id, event);
                } else {
                    // We don't forward move events without a grab
                }

                self.state.last_mouse_coord = coord;
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                self.state.last_click_button = FAKE_MOUSE_BUTTON;

                if self.mouse_grab().is_none() {
                    // If there's a mouse grab, we will continue to receive
                    // coordinates; if not, set a fake coordinate off the window
                    self.state.last_mouse_coord = Coord(-1, -1);
                    self.set_hover(widget, None);
                }
            }
            MouseWheel { delta, .. } => {
                if let Some((id, event)) = self.mouse_grab().and_then(|g| g.flush_move()) {
                    self.send_event(widget, id, event);
                }

                self.state.last_click_button = FAKE_MOUSE_BUTTON;

                let event = Event::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) => {
                        // The delta is given as a PhysicalPosition, so we need
                        // to convert to our vector type (Offset) here.
                        let coord = Coord::conv_approx(pos);
                        ScrollDelta::PixelDelta(coord.cast())
                    }
                });
                if let Some(id) = self.state.hover.clone() {
                    self.send_event(widget, id, event);
                }
            }
            MouseInput { state, button, .. } => {
                if let Some((id, event)) = self.mouse_grab().and_then(|g| g.flush_move()) {
                    self.send_event(widget, id, event);
                }

                let coord = self.state.last_mouse_coord;

                if state == ElementState::Pressed {
                    let now = Instant::now();
                    if button != self.state.last_click_button || self.state.last_click_timeout < now
                    {
                        self.state.last_click_button = button;
                        self.state.last_click_repetitions = 0;
                    }
                    self.state.last_click_repetitions += 1;
                    self.state.last_click_timeout = now + DOUBLE_CLICK_TIMEOUT;
                }

                if let Some(grab) = self.remove_mouse_grab() {
                    if grab.mode == GrabMode::Grab {
                        // Mouse grab active: send events there
                        // Note: any button release may end the grab (intended).
                        let event = Event::PressEnd {
                            source: PressSource::Mouse(grab.button, grab.repetitions),
                            end_id: self.state.hover.clone(),
                            coord,
                            success: state == ElementState::Released,
                        };
                        self.send_event(widget, grab.start_id, event);
                    }
                    // Pan events do not receive Start/End notifications
                }

                if state == ElementState::Pressed {
                    if let Some(start_id) = self.state.hover.clone() {
                        // No mouse grab but have a hover target
                        if self.state.config.mouse_nav_focus() {
                            if let Some(w) = widget.find_widget(&start_id) {
                                if w.key_nav() {
                                    self.set_nav_focus(w.id(), false);
                                }
                            }
                        }
                    }

                    let source = PressSource::Mouse(button, self.state.last_click_repetitions);
                    let event = Event::PressStart {
                        source,
                        start_id: self.state.hover.clone(),
                        coord,
                    };
                    self.send_popup_first(widget, self.state.hover.clone(), event);
                }
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            Touch(touch) => {
                let source = PressSource::Touch(touch.id);
                let coord = touch.location.cast_approx();
                match touch.phase {
                    TouchPhase::Started => {
                        let start_id = widget.find_id(coord);
                        if let Some(id) = start_id.as_ref() {
                            if self.state.config.touch_nav_focus() {
                                if let Some(w) = widget.find_widget(id) {
                                    if w.key_nav() {
                                        self.set_nav_focus(w.id(), false);
                                    }
                                }
                            }

                            let event = Event::PressStart {
                                source,
                                start_id: start_id.clone(),
                                coord,
                            };
                            self.send_popup_first(widget, start_id, event);
                        }
                    }
                    TouchPhase::Moved => {
                        let cur_id = widget.find_id(coord);

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
                            self.send_action(TkAction::REDRAW);
                        } else if let Some(pan_grab) = pan_grab {
                            if usize::conv(pan_grab.1) < MAX_PAN_GRABS {
                                if let Some(pan) =
                                    self.state.pan_grab.get_mut(usize::conv(pan_grab.0))
                                {
                                    pan.coords[usize::conv(pan_grab.1)].1 = coord;
                                }
                            }
                        }
                    }
                    ev @ (TouchPhase::Ended | TouchPhase::Cancelled) => {
                        if let Some(mut grab) = self.remove_touch(touch.id) {
                            if let Some((id, event)) = grab.flush_move() {
                                self.send_event(widget, id, event);
                            }

                            if grab.mode == GrabMode::Grab {
                                let event = Event::PressEnd {
                                    source,
                                    end_id: grab.cur_id.clone(),
                                    coord,
                                    success: ev == TouchPhase::Ended,
                                };
                                self.send_event(widget, grab.start_id, event);
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
