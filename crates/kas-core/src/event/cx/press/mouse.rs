// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: mouse events

use super::{GrabMode, Press, PressSource, velocity};
use crate::event::{Event, EventCx, EventState, FocusSource, ScrollDelta};
use crate::geom::{Affine, Coord, DVec2};
use crate::{Action, Id, NavAdvance, Node, Widget, Window};
use cast::{Cast, CastApprox, ConvApprox};
use std::time::{Duration, Instant};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::CursorIcon;

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

#[derive(Clone, Debug)]
struct PanDetails {
    c0: DVec2,
    c1: DVec2,
    moved: bool,
    mode: (bool, bool), // (scale, rotate)
}

#[derive(Clone, Debug)]
enum GrabDetails {
    Click,
    Grab,
    Pan(PanDetails),
}

impl GrabDetails {
    fn is_pan(&self) -> bool {
        matches!(self, GrabDetails::Pan(_))
    }

    /// `mode` is `(scale, rotate)`
    fn pan(position: DVec2, mode: (bool, bool)) -> Self {
        GrabDetails::Pan(PanDetails {
            c0: position,
            c1: position,
            moved: false,
            mode,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct MouseGrab {
    button: MouseButton,
    repetitions: u32,
    pub(super) start_id: Id,
    pub(super) depress: Option<Id>,
    details: GrabDetails,
    cancel: bool,
}

pub(in crate::event::cx) struct Mouse {
    pub(super) hover: Option<Id>,
    pub(super) hover_icon: CursorIcon,
    old_hover_icon: CursorIcon,
    last_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    last_pin: Option<(Id, DVec2)>,
    pub(super) grab: Option<MouseGrab>,
    last_position: DVec2,
    pub(super) samples: velocity::Samples,
}

impl Default for Mouse {
    fn default() -> Self {
        Mouse {
            hover: None,
            hover_icon: CursorIcon::Default,
            old_hover_icon: CursorIcon::Default,
            last_coord: Coord::ZERO,
            last_click_button: FAKE_MOUSE_BUTTON,
            last_click_repetitions: 0,
            last_click_timeout: Instant::now(),
            last_pin: None,
            grab: None,
            last_position: DVec2::ZERO,
            samples: Default::default(),
        }
    }
}

impl Mouse {
    /// Clear all focus and grabs on `target`
    pub(in crate::event::cx) fn cancel_event_focus(&mut self, target: &Id) {
        if let Some(grab) = self.grab.as_mut()
            && grab.start_id == target
        {
            grab.cancel = true;
        }
    }

    pub(in crate::event::cx) fn update_hover_icon(&mut self) -> Option<CursorIcon> {
        let mut icon = None;
        if self.hover_icon != self.old_hover_icon && self.grab.is_none() {
            icon = Some(self.hover_icon);
        }
        self.old_hover_icon = self.hover_icon;
        icon
    }

    pub fn frame_update(&mut self) -> Option<(Id, Affine)> {
        if let Some(grab) = self.grab.as_mut()
            && let GrabDetails::Pan(details) = &mut grab.details
        {
            // Mouse coordinates:
            let (old, new) = (details.c0, details.c1);
            details.c0 = details.c1;

            let transform = if let Some((_, y)) = self.last_pin.as_ref() {
                Affine::pan(old, new, *y, *y, details.mode)
            } else {
                Affine::translate(new - old)
            };

            if transform.is_finite() && transform != Affine::IDENTITY {
                let id = grab.start_id.clone();
                return Some((id, transform));
            }
        }

        None
    }

    pub(crate) fn hover(&self) -> Option<Id> {
        self.hover.clone()
    }

    fn update_hover(&mut self) -> (bool, bool) {
        let (mut cancel, mut redraw) = (false, false);
        if let Some(grab) = self.grab.as_mut() {
            cancel = grab.cancel;
            if let GrabDetails::Click = grab.details {
                let hover = self.hover.as_ref();
                if grab.start_id == hover {
                    if grab.depress.as_ref() != hover {
                        grab.depress = hover.cloned();
                        redraw = true;
                    }
                } else if grab.depress.is_some() {
                    grab.depress = None;
                    redraw = true;
                }
            }
        }
        (cancel, redraw)
    }

    /// Returns `true` on success
    pub(crate) fn start_grab(
        &mut self,
        button: MouseButton,
        repetitions: u32,
        id: Id,
        coord: Coord,
        mode: GrabMode,
    ) -> bool {
        let details = match mode {
            GrabMode::Click => GrabDetails::Click,
            GrabMode::Grab => GrabDetails::Grab,
            GrabMode::Pan { scale, rotate } => {
                let position = self.last_position;
                debug_assert_eq!(coord, position.cast_approx());

                // Do we have a pin?
                if matches!(&self.last_pin, Some((id2, _)) if id == *id2) {
                    GrabDetails::pan(position, (scale, rotate))
                } else {
                    GrabDetails::pan(position, (false, false))
                }
            }
        };
        if let Some(ref mut grab) = self.grab {
            if grab.start_id != id
                || grab.button != button
                || grab.details.is_pan() != mode.is_pan()
                || grab.cancel
            {
                return false;
            }

            debug_assert!(repetitions >= grab.repetitions);
            grab.repetitions = repetitions;
            grab.depress = Some(id.clone());
            grab.details = details;
        } else {
            self.grab = Some(MouseGrab {
                button,
                repetitions,
                start_id: id.clone(),
                depress: Some(id.clone()),
                details,
                cancel: false,
            });
        }
        true
    }
}

impl EventState {
    pub(crate) fn mouse_pin(&self) -> Option<(DVec2, bool)> {
        if let Some((_, position)) = self.mouse.last_pin.as_ref() {
            let used = self
                .mouse
                .grab
                .as_ref()
                .map(|grab| grab.details.is_pan())
                .unwrap_or(false);
            Some((*position, used))
        } else {
            None
        }
    }
}

impl<'a> EventCx<'a> {
    // Clear old hover, set new hover, send events.
    // If there is a popup, only permit descendands of that.
    fn set_hover(&mut self, mut widget: Node<'_>, mut w_id: Option<Id>) {
        if let Some(ref id) = w_id
            && let Some(popup) = self.popups.last()
            && !popup.1.id.is_ancestor_of(id)
        {
            w_id = None;
        }

        if self.mouse.hover != w_id {
            log::trace!("set_hover: w_id={w_id:?}");
            self.mouse.hover_icon = Default::default();
            if let Some(id) = self.mouse.hover.take() {
                self.send_event(widget.re(), id, Event::MouseHover(false));
            }
            self.mouse.hover = w_id.clone();

            if let Some(id) = w_id {
                self.send_event(widget, id, Event::MouseHover(true));
            }
        }
    }

    // Clears mouse grab and pan grab, resets cursor and redraws
    fn remove_mouse_grab(&mut self, node: Node<'_>, success: bool) {
        let mut to_send = None;
        let last_pin;
        let redraw;
        if let Some(grab) = self.mouse.grab.as_ref() {
            log::trace!(
                "remove_mouse_grab: start_id={}, success={success}",
                grab.start_id
            );
            self.window.set_cursor_icon(self.mouse.hover_icon);
            redraw = grab.depress.clone();
            if let GrabDetails::Pan(details) = &grab.details {
                if success && !details.moved {
                    last_pin = Some((grab.start_id.clone(), self.mouse.last_position));
                } else {
                    last_pin = None;
                }
                // Pan grabs do not receive Event::PressEnd
            } else {
                last_pin = None;
                let press = Press {
                    source: PressSource::Mouse(grab.button, grab.repetitions),
                    id: self.mouse.hover.clone(),
                    coord: self.mouse.last_coord,
                };
                let event = Event::PressEnd { press, success };
                to_send = Some((grab.start_id.clone(), event));
            }
        } else {
            return;
        }

        // We must send Event::PressEnd before removing the grab
        if let Some((id, event)) = to_send {
            self.send_event(node, id, event);
        }
        self.mouse.last_pin = last_pin;
        self.opt_action(redraw, Action::REDRAW);

        self.mouse.grab = None;
    }

    pub(in crate::event::cx) fn mouse_handle_pending<A>(&mut self, win: &mut Window<A>, data: &A) {
        let (cancel, redraw) = self.mouse.update_hover();
        if cancel {
            self.remove_mouse_grab(win.as_node(data), false);
        }

        if redraw {
            self.action |= Action::REDRAW;
        }

        if self.action.contains(Action::REGION_MOVED) {
            // Update hovered widget
            let hover = win.try_probe(self.mouse.last_coord);
            self.set_hover(win.as_node(data), hover);
        }
    }

    /// Handle mouse cursor motion.
    pub(in crate::event::cx) fn handle_cursor_moved<A>(
        &mut self,
        win: &mut Window<A>,
        data: &A,
        position: DVec2,
    ) {
        let delta = position - self.mouse.last_position;
        self.mouse.samples.push_delta(delta.cast_approx());
        self.mouse.last_position = position;
        self.mouse.last_click_button = FAKE_MOUSE_BUTTON;
        let coord = position.cast_approx();

        // Update hovered win
        let id = win.try_probe(coord);
        self.set_hover(win.as_node(data), id.clone());

        if let Some(grab) = self.mouse.grab.as_mut() {
            match &mut grab.details {
                GrabDetails::Click => (),
                GrabDetails::Grab => {
                    let target = grab.start_id.clone();
                    let press = Press {
                        source: PressSource::Mouse(grab.button, grab.repetitions),
                        id,
                        coord,
                    };
                    let delta = coord - self.mouse.last_coord;
                    let event = Event::PressMove { press, delta };
                    self.send_event(win.as_node(data), target, event);
                }
                GrabDetails::Pan(details) => {
                    details.c1 = position;
                    details.moved = true;
                    self.need_frame_update = true;
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

        self.mouse.last_coord = coord;
    }

    /// Handle mouse cursor entering the app.
    #[inline]
    pub(in crate::event::cx) fn handle_cursor_entered(&mut self) {}

    /// Handle mouse cursor leaving the app.
    pub(in crate::event::cx) fn handle_cursor_left(&mut self, node: Node<'_>) {
        self.mouse.last_click_button = FAKE_MOUSE_BUTTON;

        if self.mouse.grab.is_none() {
            // If there's a mouse grab, we will continue to receive
            // coordinates; if not, set a fake coordinate off the window
            self.mouse.last_coord = Coord(-1, -1);
            self.set_hover(node, None);
        }
    }

    /// Handle a mouse wheel event.
    pub(in crate::event::cx) fn handle_mouse_wheel(
        &mut self,
        node: Node<'_>,
        delta: MouseScrollDelta,
    ) {
        self.mouse.last_click_button = FAKE_MOUSE_BUTTON;

        let event = Event::Scroll(match delta {
            MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines(x, y),
            MouseScrollDelta::PixelDelta(pos) => {
                // The delta is given as a PhysicalPosition, so we need
                // to convert to our vector type (Offset) here.
                let coord = Coord::conv_approx(pos);
                ScrollDelta::Pixels(coord.cast())
            }
        });
        if let Some(id) = self.mouse.hover.clone() {
            self.send_event(node, id, event);
        }
    }

    /// Handle a mouse click / release.
    pub(in crate::event::cx) fn handle_mouse_input(
        &mut self,
        mut node: Node<'_>,
        state: ElementState,
        button: MouseButton,
    ) {
        if state == ElementState::Pressed {
            let now = Instant::now();
            if button != self.mouse.last_click_button || self.mouse.last_click_timeout < now {
                self.mouse.last_click_button = button;
                self.mouse.last_click_repetitions = 0;
            }
            self.mouse.last_click_repetitions += 1;
            self.mouse.last_click_timeout = now + DOUBLE_CLICK_TIMEOUT;
        }

        if self
            .mouse
            .grab
            .as_ref()
            .map(|g| g.button == button)
            .unwrap_or(false)
        {
            self.remove_mouse_grab(node.re(), true);
        }

        if state == ElementState::Pressed {
            if let Some(start_id) = self.mouse.hover.clone() {
                // No mouse grab but have a hover target
                if matches!(self.mouse.last_pin.as_ref(), Some((id, _)) if *id != start_id) {
                    self.mouse.last_pin = None;
                }
                if self.config.event().mouse_nav_focus()
                    && let Some(id) = self.nav_next(node.re(), Some(&start_id), NavAdvance::None)
                {
                    self.set_nav_focus(id, FocusSource::Pointer);
                }
            }

            let source = PressSource::Mouse(button, self.mouse.last_click_repetitions);
            let press = Press {
                source,
                id: self.mouse.hover.clone(),
                coord: self.mouse.last_coord,
            };
            let event = Event::PressStart { press };
            self.send_popup_first(node, self.mouse.hover.clone(), event);
        }
    }
}
