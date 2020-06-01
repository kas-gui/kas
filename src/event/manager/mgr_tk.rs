// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager — toolkit API

use log::*;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::Instant;

use super::*;
use crate::geom::{Coord, DVec2};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{TkAction, TkWindow, Widget, WidgetId};

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl ManagerState {
    /// Construct an event manager per-window data struct
    ///
    /// The DPI factor may be required for event coordinate translation.
    #[inline]
    pub fn new(dpi_factor: f64) -> Self {
        ManagerState {
            end_id: Default::default(),
            dpi_factor,
            modifiers: ModifiersState::empty(),
            char_focus: None,
            nav_focus: None,
            nav_fallback: None,
            nav_stack: SmallVec::new(),
            hover: None,
            hover_icon: CursorIcon::Default,
            key_depress: Default::default(),
            last_mouse_coord: Coord::ZERO,
            mouse_grab: None,
            touch_grab: Default::default(),
            pan_grab: SmallVec::new(),
            accel_stack: vec![],
            accel_layers: HashMap::new(),
            popups: Default::default(),
            new_popups: Default::default(),
            popup_removed: Default::default(),

            time_start: Instant::now(),
            time_updates: vec![],
            handle_updates: HashMap::new(),
            pending: SmallVec::new(),
            action: TkAction::None,
        }
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    pub fn configure<W>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        trace!("Manager::configure");
        self.action = TkAction::None;

        // Re-assigning WidgetIds might invalidate state; to avoid this we map
        // existing ids to new ids
        let mut map = HashMap::new();
        let mut id = WidgetId::FIRST;

        // We re-set these instead of remapping:
        self.accel_stack.clear();
        self.accel_layers.clear();
        self.time_updates.clear();
        self.handle_updates.clear();
        self.pending.clear();
        self.nav_fallback = None;

        // Enumerate and configure all widgets:
        let coord = self.last_mouse_coord;
        self.with(tkw, |mut mgr| {
            mgr.push_accel_layer(false);
            widget.configure_recurse(ConfigureManager {
                id: &mut id,
                map: &mut map,
                mgr: &mut mgr,
            });
            mgr.pop_accel_layer(widget.id());
            debug_assert!(mgr.mgr.accel_stack.is_empty());

            let hover = widget.find_id(coord);
            mgr.set_hover(widget, hover);
        });
        if self.action == TkAction::Reconfigure {
            warn!("Detected TkAction::Reconfigure during configure. This may cause a reconfigure-loop.");
            if id == self.end_id {
                panic!("Reconfigure occurred with the same number of widgets — we are probably stuck in a reconfigure-loop.");
            }
            self.end_id = id;
        }

        // The remaining code just updates all input states to new IDs via the map.

        self.char_focus = self.char_focus.and_then(|id| map.get(&id).cloned());
        self.nav_focus = self.nav_focus.and_then(|id| map.get(&id).cloned());
        self.mouse_grab = self.mouse_grab.as_ref().and_then(|grab| {
            map.get(&grab.start_id).map(|id| MouseGrab {
                start_id: *id,
                depress: grab.depress.and_then(|id| map.get(&id).cloned()),
                button: grab.button,
                mode: grab.mode,
                pan_grab: grab.pan_grab,
            })
        });

        let mut i = 0;
        while i < self.pan_grab.len() {
            if let Some(id) = map.get(&self.pan_grab[i].id) {
                self.pan_grab[i].id = *id;
                i += 1;
            } else {
                self.remove_pan(i);
            }
        }
        macro_rules! do_map {
            ($seq:expr, $update:expr) => {
                let update = $update;
                let mut i = 0;
                let mut j = $seq.len();
                while i < j {
                    // invariant: $seq[0..i] have been updated
                    // invariant: $seq[j..len] are rejected
                    if let Some(elt) = update($seq[i].clone()) {
                        $seq[i] = elt;
                        i += 1;
                    } else {
                        j -= 1;
                        $seq.swap(i, j);
                    }
                }
                $seq.truncate(j);
            };
        }

        do_map!(self.touch_grab, |mut elt: TouchGrab| map
            .get(&elt.start_id)
            .map(|id| {
                elt.start_id = *id;
                if let Some(cur_id) = elt.cur_id {
                    elt.cur_id = map.get(&cur_id).cloned();
                }
                elt
            }));

        do_map!(self.key_depress, |elt: (u32, WidgetId)| map
            .get(&elt.1)
            .map(|id| (elt.0, *id)));
    }

    /// Update the widgets under the cursor and touch events
    pub fn region_moved<W: Widget + ?Sized>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W) {
        trace!("Manager::region_moved");
        // Note: redraw is already implied.

        self.nav_focus = self
            .nav_focus
            .and_then(|id| widget.find(id).map(|w| w.id()));
        self.char_focus = self
            .char_focus
            .and_then(|id| widget.find(id).map(|w| w.id()));

        // Update hovered widget
        let hover = widget.find_id(self.last_mouse_coord);
        self.with(tkw, |mgr| mgr.set_hover(widget, hover));

        for touch in &mut self.touch_grab {
            touch.cur_id = widget.find_id(touch.coord);
        }
    }

    /// Set the DPI factor. Must be updated for correct event translation by
    /// [`Manager::handle_winit`].
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

    /// Get the next resume time
    pub fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    /// Set an action
    ///
    /// Since this is a commonly used operation, an operator overload is
    /// available to do this job: `*mgr += action;`.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    /// Construct a [`Manager`] referring to this state
    ///
    /// Invokes the given closure on this [`Manager`].
    #[inline]
    pub fn with<F>(&mut self, tkw: &mut dyn TkWindow, f: F)
    where
        F: FnOnce(&mut Manager),
    {
        let mut mgr = Manager {
            read_only: false,
            mgr: self,
            tkw,
            action: TkAction::None,
        };
        f(&mut mgr);
        let action = mgr.action;
        self.send_action(action);
    }

    /// Update, after receiving all events
    #[inline]
    pub fn update<W>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W) -> TkAction
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        let mut mgr = Manager {
            read_only: false,
            mgr: self,
            tkw,
            action: TkAction::None,
        };

        while let Some((parent, wid)) = mgr.mgr.popup_removed.pop() {
            mgr.send_event(widget, parent, Event::PopupRemoved(wid));
        }
        while let Some(id) = mgr.mgr.new_popups.pop() {
            for parent in mgr
                .mgr
                .popups
                .iter()
                .map(|(_, popup)| popup.parent)
                .collect::<SmallVec<[WidgetId; 16]>>()
            {
                mgr.send_event(widget, parent, Event::NewPopup(id));
            }
        }

        for gi in 0..mgr.mgr.pan_grab.len() {
            let grab = &mut mgr.mgr.pan_grab[gi];
            debug_assert!(grab.mode != GrabMode::Grab);
            assert!(grab.n > 0);

            // Terminology: pi are old coordinates, qi are new coords
            let (p1, q1) = (DVec2::from(grab.coords[0].0), DVec2::from(grab.coords[0].1));
            grab.coords[0].0 = grab.coords[0].1;

            let alpha;
            let delta;

            if grab.mode == GrabMode::PanOnly || grab.n == 1 {
                alpha = DVec2(1.0, 0.0);
                delta = DVec2::from(q1 - p1);
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

        // To avoid infinite loops, we consider mgr read-only from here on.
        // Since we don't wish to duplicate Handler::handle, we don't actually
        // make mgr const, but merely pretend it is in the public API.
        mgr.read_only = true;

        for item in mgr.mgr.pending.pop() {
            match item {
                Pending::LostCharFocus(id) => {
                    let event = Event::LostCharFocus;
                    mgr.send_event(widget, id, event);
                }
            }
        }

        let mut action = mgr.action;
        action += self.action;
        self.action = TkAction::None;
        action
    }
}

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<'a> Manager<'a> {
    /// Update widgets due to timer
    pub fn update_timer<W: Widget + ?Sized>(&mut self, widget: &mut W) {
        let now = Instant::now();

        // assumption: time_updates are sorted in reverse order
        while !self.mgr.time_updates.is_empty() {
            if self.mgr.time_updates.last().unwrap().0 > now {
                break;
            }

            let update = self.mgr.time_updates.pop().unwrap();
            self.send_event(widget, update.1, Event::TimerUpdate);
        }

        self.mgr.time_updates.sort_by(|a, b| b.cmp(a)); // reverse sort
    }

    /// Update widgets due to handle
    pub fn update_handle<W: Widget + ?Sized>(
        &mut self,
        widget: &mut W,
        handle: UpdateHandle,
        payload: u64,
    ) {
        // NOTE: to avoid borrow conflict, we must clone values!
        if let Some(mut values) = self.mgr.handle_updates.get(&handle).cloned() {
            for w_id in values.drain(..) {
                let event = Event::HandleUpdate { handle, payload };
                self.send_event(widget, w_id, event);
            }
        }
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(&mut self, widget: &mut W, event: winit::event::WindowEvent)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};

        // Note: since <W as Handler>::Msg = VoidMsg, only two values of
        // Response are possible: None and Unhandled. We don't have any use for
        // Unhandled events here, so we can freely ignore all responses.

        match event {
            CloseRequested => self.send_action(TkAction::Close),
            /* Not yet supported: see #98
            DroppedFile(path) => ,
            HoveredFile(path) => ,
            HoveredFileCancelled => ,
            */
            ReceivedCharacter(c) if c != '\u{1b}' /* escape */ => {
                if let Some(id) = self.mgr.char_focus {
                    let event = Event::ReceivedCharacter(c);
                    self.send_event(widget, id, event);
                }
            }
            KeyboardInput { input, is_synthetic, .. } => {
                if input.state == ElementState::Pressed && !is_synthetic {
                    if let Some(vkey) = input.virtual_keycode {
                        self.start_key_event(widget, vkey, input.scancode);
                    }
                } else if input.state == ElementState::Released {
                    self.end_key_event(input.scancode);
                }
            }
            ModifiersChanged(state) => {
                if state.alt() != self.mgr.modifiers.alt() {
                    // This controls drawing of accelerator key indicators
                    self.mgr.send_action(TkAction::Redraw);
                }
                self.mgr.modifiers = state;
            }
            CursorMoved {
                position,
                ..
            } => {
                let coord = position.into();

                // Update hovered widget
                let cur_id = widget.find_id(coord);
                let delta = coord - self.mgr.last_mouse_coord;
                self.set_hover(widget, cur_id);

                if let Some(grab) = self.mouse_grab() {
                    if grab.mode == GrabMode::Grab {
                        let source = PressSource::Mouse(grab.button);
                        let event = Event::PressMove { source, cur_id, coord, delta };
                        self.send_event(widget, grab.start_id, event);
                    } else if let Some(pan) = self.mgr.pan_grab.get_mut(grab.pan_grab.0 as usize) {
                        pan.coords[grab.pan_grab.1 as usize].1 = coord;
                    }
                } else if let Some(id) = self.mgr.popups.last().map(|(_, p)| p.parent) {
                    // Use a fake button!
                    let source = PressSource::Mouse(MouseButton::Other(0));
                    let event = Event::PressMove { source, cur_id, coord, delta };
                    self.send_event(widget, id, event);
                } else {
                    // We don't forward move events without a grab
                }

                self.mgr.last_mouse_coord = coord;
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                if self.mouse_grab().is_none() {
                    // If there's a mouse grab, we will continue to receive
                    // coordinates; if not, set a fake coordinate off the window
                    self.mgr.last_mouse_coord = Coord(-1, -1);
                    self.set_hover(widget, None);
                }
            }
            MouseWheel { delta, .. } => {
                let event = Event::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) =>
                        ScrollDelta::PixelDelta(Coord::from_logical(pos, self.mgr.dpi_factor)),
                });
                if let Some(id) = self.mgr.hover {
                    self.send_event(widget, id, event);
                }
            }
            MouseInput {
                state,
                button,
                ..
            } => {
                let coord = self.mgr.last_mouse_coord;
                let source = PressSource::Mouse(button);

                if let Some(grab) = self.mouse_grab() {
                    match grab.mode {
                        GrabMode::Grab => {
                            // Mouse grab active: send events there
                            debug_assert_eq!(state, ElementState::Released);
                            let event = Event::PressEnd {
                                source,
                                end_id: self.mgr.hover,
                                coord,
                            };
                            self.send_event(widget, grab.start_id, event);
                        }
                        // Pan events do not receive Start/End notifications
                        _ => (),
                    };

                    if state == ElementState::Released {
                        self.end_mouse_grab(button);
                    }
                } else if let Some(start_id) = self.mgr.hover {
                    // No mouse grab but have a hover target
                    if state == ElementState::Pressed {
                        let event = Event::PressStart { source, start_id, coord };
                        self.send_popup_first(widget, start_id, event);
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
                            let event = Event::PressStart { source, start_id, coord };
                            self.send_popup_first(widget, start_id, event);
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
                                let redraw = grab.cur_id != cur_id &&
                                    (grab.cur_id == Some(grab.start_id) || cur_id == Some(grab.start_id));

                                grab.cur_id = cur_id;
                                grab.coord = coord;

                                r = Some((id, event, redraw));
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if let Some((id, event, redraw)) = r {
                            if redraw {
                                self.send_action(TkAction::Redraw);
                            }
                            self.send_event(widget, id, event);
                        } else if let Some(pan_grab) = pan_grab {
                            if (pan_grab.1 as usize) < MAX_PAN_GRABS {
                                if let Some(pan) = self.mgr.pan_grab.get_mut(pan_grab.0 as usize) {
                                    pan.coords[pan_grab.1 as usize].1 = coord;
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
                                self.mgr.remove_pan_grab(grab.pan_grab);
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
