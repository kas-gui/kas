// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — shell API

use log::*;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::mem::swap;
use std::time::{Duration, Instant};

use super::*;
use crate::cast::Conv;
use crate::geom::{Coord, DVec2, Offset};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{ShellWindow, TkAction, Widget, WidgetId};

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl ManagerState {
    /// Construct an event manager per-window data struct
    #[inline]
    pub fn new(config: Rc<RefCell<Config>>, scale_factor: f32) -> Self {
        ManagerState {
            config,
            scale_factor,
            widget_count: 0,
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
            accel_stack: vec![],
            accel_layers: HashMap::new(),
            popups: Default::default(),
            new_popups: Default::default(),
            popup_removed: Default::default(),
            time_updates: vec![],
            handle_updates: HashMap::new(),
            pending: SmallVec::new(),
            action: TkAction::empty(),
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    ///
    /// This method calls [`WidgetConfig::configure_recurse`] in order to assign
    /// [`WidgetId`] identifiers and call widgets' [`WidgetConfig::configure`]
    /// method. Additionally, it updates the [`ManagerState`] to account for
    /// renamed and removed widgets.
    pub fn configure<W>(&mut self, shell: &mut dyn ShellWindow, widget: &mut W)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        debug!("Manager::configure");
        self.action = TkAction::empty();

        // Re-assigning WidgetIds might invalidate state; to avoid this we map
        // existing ids to new ids
        let mut renames = HashMap::new();
        let mut count = 0;
        let id = WidgetId::ROOT;

        // We re-create these instead of renaming IDs:
        debug_assert!(self.accel_stack.is_empty());
        self.accel_stack.clear();
        self.accel_layers.clear();
        self.nav_fallback = None;
        // These we merge later:
        let mut old_time_updates = Default::default();
        swap(&mut self.time_updates, &mut old_time_updates);
        let mut old_handle_updates = Default::default();
        swap(&mut self.handle_updates, &mut old_handle_updates);

        // Enumerate and configure all widgets:
        let coord = self.last_mouse_coord;
        self.with(shell, |mgr| {
            mgr.push_accel_layer(false);
            widget.configure_recurse(ConfigureManager {
                count: &mut count,
                used: false,
                id,
                map: &mut renames,
                mgr,
            });
            mgr.pop_accel_layer(widget.id());
            debug_assert!(mgr.state.accel_stack.is_empty());

            let hover = widget.find_id(coord);
            mgr.set_hover(widget, hover);
        });
        if self.action.contains(TkAction::RECONFIGURE) {
            warn!("Detected TkAction::RECONFIGURE during configure. This may cause a reconfigure-loop.");
            if count == self.widget_count {
                panic!("Reconfigure occurred with the same number of widgets — we are probably stuck in a reconfigure-loop.");
            }
            self.widget_count = count;
        }

        // Update input state to account for renamed widgets. Assumption: none
        // of this state is adjusted within widget configure methods.
        // TODO(safety): ensure these fields cannot be updated by configure?

        self.sel_focus = self.sel_focus.and_then(|id| renames.get(&id).cloned());
        self.nav_focus = self.nav_focus.and_then(|id| renames.get(&id).cloned());
        self.mouse_grab = self.mouse_grab.as_ref().and_then(|grab| {
            renames.get(&grab.start_id).map(|id| MouseGrab {
                button: grab.button,
                repetitions: grab.repetitions,
                start_id: *id,
                depress: grab.depress.and_then(|id| renames.get(&id).cloned()),
                mode: grab.mode,
                pan_grab: grab.pan_grab,
            })
        });

        let mut i = 0;
        while i < self.pan_grab.len() {
            if let Some(id) = renames.get(&self.pan_grab[i].id) {
                self.pan_grab[i].id = *id;
                i += 1;
            } else {
                self.remove_pan(i);
            }
        }

        self.touch_grab.retain(|_, grab| {
            if let Some(id) = renames.get(&grab.start_id) {
                grab.start_id = *id;
                if let Some(cur_id) = grab.cur_id {
                    grab.cur_id = renames.get(&cur_id).cloned();
                }
                true
            } else {
                false
            }
        });

        self.key_depress.retain(|_, depress_id| {
            if let Some(id) = renames.get(depress_id) {
                *depress_id = *id;
                true
            } else {
                false
            }
        });

        // We have to handle time_updates and handle_updates carefully since
        // these may be set during configure, *and* may carry old state forward
        // which must be renamed.
        'old: for (time, old_id, payload) in old_time_updates.drain(..) {
            if let Some(new_id) = renames.get(&old_id).cloned() {
                // Insert into our data structure. We sort everything below.
                'insert: loop {
                    for row in &mut self.time_updates {
                        if row.1 == new_id {
                            if row.0 <= time {
                                continue 'old;
                            } else {
                                row.0 = time;
                                break 'insert;
                            }
                        }
                    }

                    self.time_updates.push((time, new_id, payload));
                    break;
                }
            }
        }
        self.time_updates.sort_by(|a, b| b.0.cmp(&a.0)); // reverse sort

        for (handle, mut ids) in old_handle_updates.drain() {
            let new_ids = self
                .handle_updates
                .entry(handle)
                .or_insert_with(Default::default);
            for id in ids.drain().filter_map(|id| renames.get(&id)).cloned() {
                new_ids.insert(id);
            }
        }

        self.pending.retain(|item| match item {
            Pending::LostCharFocus(id) => {
                if let Some(new_id) = renames.get(id) {
                    *item = Pending::LostCharFocus(*new_id);
                    true
                } else {
                    false
                }
            }
            Pending::LostSelFocus(id) => {
                if let Some(new_id) = renames.get(id) {
                    *item = Pending::LostSelFocus(*new_id);
                    true
                } else {
                    false
                }
            }
            Pending::SetNavFocus(id, key_focus) => {
                if let Some(new_id) = renames.get(id) {
                    *item = Pending::SetNavFocus(*new_id, *key_focus);
                    true
                } else {
                    false
                }
            }
        });
    }

    /// Update the widgets under the cursor and touch events
    pub fn region_moved<W: Widget + ?Sized>(
        &mut self,
        shell: &mut dyn ShellWindow,
        widget: &mut W,
    ) {
        trace!("Manager::region_moved");
        // Note: redraw is already implied.

        // Update hovered widget
        let hover = widget.find_id(self.last_mouse_coord);
        self.with(shell, |mgr| mgr.set_hover(widget, hover));

        for grab in self.touch_grab.iter_mut() {
            grab.1.cur_id = widget.find_id(grab.1.coord);
        }
    }

    /// Get the next resume time
    pub fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    /// Set an action
    ///
    /// Since this is a commonly used operation, an operator overload is
    /// available to do this job: `*mgr |= action;`.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    /// Construct a [`Manager`] referring to this state
    ///
    /// Invokes the given closure on this [`Manager`].
    #[inline]
    pub fn with<F>(&mut self, shell: &mut dyn ShellWindow, f: F)
    where
        F: FnOnce(&mut Manager),
    {
        let mut mgr = Manager {
            state: self,
            shell,
            action: TkAction::empty(),
        };
        f(&mut mgr);
        let action = mgr.action;
        self.send_action(action);
    }

    /// Update, after receiving all events
    #[inline]
    pub fn update<W>(&mut self, shell: &mut dyn ShellWindow, widget: &mut W) -> TkAction
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        let mut mgr = Manager {
            state: self,
            shell,
            action: TkAction::empty(),
        };

        while let Some((parent, wid)) = mgr.state.popup_removed.pop() {
            mgr.send_event(widget, parent, Event::PopupRemoved(wid));
        }
        while let Some(id) = mgr.state.new_popups.pop() {
            while let Some((_, popup, _)) = mgr.state.popups.last() {
                if widget
                    .find_leaf(popup.parent)
                    .map(|w| w.is_ancestor_of(id))
                    .unwrap_or(false)
                {
                    break;
                }
                let (wid, popup, _old_nav_focus) = mgr.state.popups.pop().unwrap();
                mgr.send_event(widget, popup.parent, Event::PopupRemoved(wid));
                // Don't restore old nav focus: assume new focus will be set by new popup
            }
        }

        for gi in 0..mgr.state.pan_grab.len() {
            let grab = &mut mgr.state.pan_grab[gi];
            debug_assert!(grab.mode != GrabMode::Grab);
            assert!(grab.n > 0);

            // Terminology: pi are old coordinates, qi are new coords
            let (p1, q1) = (DVec2::from(grab.coords[0].0), DVec2::from(grab.coords[0].1));
            grab.coords[0].0 = grab.coords[0].1;

            let alpha;
            let delta;

            if grab.mode == GrabMode::PanOnly || grab.n == 1 {
                alpha = DVec2(1.0, 0.0);
                delta = q1 - p1;
            } else {
                // We don't use more than two touches: information would be
                // redundant (although it could be averaged).
                let (p2, q2) = (DVec2::from(grab.coords[1].0), DVec2::from(grab.coords[1].1));
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

            let id = grab.id;
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

        let action = mgr.action | self.action;
        self.action = TkAction::empty();
        action
    }
}

/// Shell API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
impl<'a> Manager<'a> {
    /// Update widgets due to timer
    pub fn update_timer<W: Widget + ?Sized>(&mut self, widget: &mut W) {
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
    pub fn update_handle<W: Widget + ?Sized>(
        &mut self,
        widget: &mut W,
        handle: UpdateHandle,
        payload: u64,
    ) {
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
    pub fn handle_winit<W>(&mut self, widget: &mut W, event: winit::event::WindowEvent)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};

        // Note: since <W as Handler>::Msg = VoidMsg, only two values of
        // Response are possible: None and Unhandled. We don't have any use for
        // Unhandled events here, so we can freely ignore all responses.

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
            Focused(false) => {
                // Window focus lost: close all popups
                while let Some(id) = self.state.popups.last().map(|(id, _, _)| *id) {
                    self.close_window(id, true);
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
                let coord = position.into();

                // Update hovered widget
                let cur_id = widget.find_id(coord);
                let delta = coord - self.state.last_mouse_coord;
                self.set_hover(widget, cur_id);

                if let Some(grab) = self.mouse_grab() {
                    if grab.mode == GrabMode::Grab {
                        let source = PressSource::Mouse(grab.button, grab.repetitions);
                        let event = Event::PressMove {
                            source,
                            cur_id,
                            coord,
                            delta,
                        };
                        self.send_event(widget, grab.start_id, event);
                    } else if let Some(pan) =
                        self.state.pan_grab.get_mut(usize::conv(grab.pan_grab.0))
                    {
                        pan.coords[usize::conv(grab.pan_grab.1)].1 = coord;
                    }
                } else if let Some(id) = self.state.popups.last().map(|(_, p, _)| p.parent) {
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
                self.state.last_click_button = FAKE_MOUSE_BUTTON;

                let event = Event::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) => {
                        // The delta is given as a PhysicalPosition, so we need
                        // to convert to our vector type (Offset) here.
                        let coord = Coord::from(pos);
                        ScrollDelta::PixelDelta(Offset(coord.0, coord.1))
                    }
                });
                if let Some(id) = self.state.hover {
                    self.send_event(widget, id, event);
                }
            }
            MouseInput { state, button, .. } => {
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

                if let Some(grab) = self.mouse_grab() {
                    if grab.mode == GrabMode::Grab {
                        // Mouse grab active: send events there
                        debug_assert_eq!(state, ElementState::Released);
                        let source = PressSource::Mouse(button, grab.repetitions);
                        let event = Event::PressEnd {
                            source,
                            end_id: self.state.hover,
                            coord,
                        };
                        self.send_event(widget, grab.start_id, event);
                        // Pan events do not receive Start/End notifications
                    };

                    if state == ElementState::Released {
                        self.end_mouse_grab(button);
                    }
                } else if let Some(start_id) = self.state.hover {
                    // No mouse grab but have a hover target
                    if state == ElementState::Pressed {
                        let source = PressSource::Mouse(button, self.state.last_click_repetitions);
                        let event = Event::PressStart {
                            source,
                            start_id,
                            coord,
                        };
                        self.send_popup_first(widget, start_id, event);

                        if self.state.config.borrow().mouse_nav_focus() {
                            if let Some(w) = widget.find_leaf(start_id) {
                                if w.key_nav() {
                                    self.set_nav_focus(w.id(), false);
                                }
                            }
                        }
                    }
                }
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            Touch(touch) => {
                let source = PressSource::Touch(touch.id);
                let coord = touch.location.into();
                match touch.phase {
                    TouchPhase::Started => {
                        if let Some(start_id) = widget.find_id(coord) {
                            let event = Event::PressStart {
                                source,
                                start_id,
                                coord,
                            };
                            self.send_popup_first(widget, start_id, event);

                            if self.state.config.borrow().touch_nav_focus() {
                                if let Some(w) = widget.find_leaf(start_id) {
                                    if w.key_nav() {
                                        self.set_nav_focus(w.id(), false);
                                    }
                                }
                            }
                        }
                    }
                    TouchPhase::Moved => {
                        let cur_id = widget.find_id(coord);

                        let mut r = None;
                        let mut pan_grab = None;
                        if let Some(grab) = self.get_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                let id = grab.start_id;
                                let event = Event::PressMove {
                                    source,
                                    cur_id,
                                    coord,
                                    delta: coord - grab.coord,
                                };
                                // Only when 'depressed' status changes:
                                let redraw = grab.cur_id != cur_id
                                    && (grab.cur_id == Some(grab.start_id)
                                        || cur_id == Some(grab.start_id));

                                grab.cur_id = cur_id;
                                grab.coord = coord;

                                r = Some((id, event, redraw));
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if let Some((id, event, redraw)) = r {
                            if redraw {
                                self.send_action(TkAction::REDRAW);
                            }
                            self.send_event(widget, id, event);
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
                    TouchPhase::Ended => {
                        if let Some(grab) = self.remove_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                let event = Event::PressEnd {
                                    source,
                                    end_id: grab.cur_id,
                                    coord,
                                };
                                if let Some(cur_id) = grab.cur_id {
                                    self.redraw(cur_id);
                                }
                                self.send_event(widget, grab.start_id, event);
                            } else {
                                self.state.remove_pan_grab(grab.pan_grab);
                            }
                        }
                    }
                    TouchPhase::Cancelled => {
                        if let Some(grab) = self.remove_touch(touch.id) {
                            let event = Event::PressEnd {
                                source,
                                end_id: None,
                                coord,
                            };
                            if let Some(cur_id) = grab.cur_id {
                                self.redraw(cur_id);
                            }
                            self.send_event(widget, grab.start_id, event);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
