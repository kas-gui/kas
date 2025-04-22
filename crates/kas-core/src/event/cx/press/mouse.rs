// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: mouse events

use super::{GrabDetails, MouseGrab, Press, PressSource};
use crate::event::{Event, EventCx, FocusSource, ScrollDelta};
use crate::geom::{Coord, DVec2};
use crate::{Action, Id, NavAdvance, Node, Widget, Window};
use cast::{Cast, Conv, ConvApprox};
use std::time::{Duration, Instant};
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::window::CursorIcon;

// TODO: this should be configurable or derived from the system
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_secs(1);

const FAKE_MOUSE_BUTTON: MouseButton = MouseButton::Other(0);

pub(in crate::event::cx) struct Mouse {
    pub(super) hover: Option<Id>,
    pub(super) hover_icon: CursorIcon,
    old_hover_icon: CursorIcon,
    last_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    pub(super) mouse_grab: Option<MouseGrab>,
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
            mouse_grab: None,
        }
    }
}

impl Mouse {
    /// Clear all focus and grabs on `target`
    pub(in crate::event::cx) fn cancel_event_focus(&mut self, target: &Id) {
        if let Some(grab) = self.mouse_grab.as_mut() {
            if grab.start_id == target {
                grab.cancel = true;
            }
        }
    }

    pub(in crate::event::cx) fn update_hover_icon(&mut self) -> Option<CursorIcon> {
        let mut icon = None;
        if self.hover_icon != self.old_hover_icon && self.mouse_grab.is_none() {
            icon = Some(self.hover_icon);
        }
        self.old_hover_icon = self.hover_icon;
        icon
    }

    pub fn frame_update(&mut self) -> Option<(Id, Event)> {
        if let Some(grab) = self.mouse_grab.as_mut() {
            if grab.details.is_pan() {
                // Terminology: pi are old coordinates, qi are new coords
                let (p1, q1) = (DVec2::conv(grab.coords.0), DVec2::conv(grab.coords.1));
                grab.coords.0 = grab.coords.1;

                let delta = q1 - p1;
                if delta != DVec2::ZERO {
                    let id = grab.start_id.clone();
                    let alpha = DVec2(1.0, 0.0);
                    let event = Event::Pan { alpha, delta };
                    return Some((id, event));
                }
            }
        }

        None
    }

    pub(crate) fn hover(&self) -> Option<Id> {
        self.hover.clone()
    }

    fn update_hover(&mut self) -> (bool, bool) {
        let (mut cancel, mut redraw) = (false, false);
        if let Some(grab) = self.mouse_grab.as_mut() {
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
}

impl<'a> EventCx<'a> {
    // Clear old hover, set new hover, send events.
    // If there is a popup, only permit descendands of that.
    fn set_hover(&mut self, mut widget: Node<'_>, mut w_id: Option<Id>) {
        if let Some(ref id) = w_id {
            if let Some(popup) = self.popups.last() {
                if !popup.1.id.is_ancestor_of(id) {
                    w_id = None;
                }
            }
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
    fn remove_mouse_grab(&mut self, success: bool) -> Option<(Id, Event)> {
        if let Some(grab) = self.mouse.mouse_grab.take() {
            log::trace!(
                "remove_mouse_grab: start_id={}, success={success}",
                grab.start_id
            );
            self.window.set_cursor_icon(self.mouse.hover_icon);
            self.opt_action(grab.depress.clone(), Action::REDRAW);
            if grab.details.is_pan() {
                // Pan grabs do not receive Event::PressEnd
                None
            } else {
                let press = Press {
                    source: PressSource::Mouse(grab.button, grab.repetitions),
                    id: self.mouse.hover.clone(),
                    coord: self.mouse.last_coord,
                };
                let event = Event::PressEnd { press, success };
                Some((grab.start_id, event))
            }
        } else {
            None
        }
    }

    pub(in crate::event::cx) fn mouse_handle_pending<A>(&mut self, win: &mut Window<A>, data: &A) {
        let (cancel, redraw) = self.mouse.update_hover();
        if cancel {
            if let Some((id, event)) = self.remove_mouse_grab(false) {
                self.send_event(win.as_node(data), id, event);
            }
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
        coord: Coord,
    ) {
        self.mouse.last_click_button = FAKE_MOUSE_BUTTON;

        // Update hovered win
        let id = win.try_probe(coord);
        self.set_hover(win.as_node(data), id.clone());

        if let Some(grab) = self.mouse.mouse_grab.as_mut() {
            match grab.details {
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
                GrabDetails::Pan => {
                    grab.coords.1 = coord;
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

        if self.mouse.mouse_grab.is_none() {
            // If there's a mouse grab, we will continue to receive
            // coordinates; if not, set a fake coordinate off the window
            self.mouse.last_coord = Coord(-1, -1);
            self.set_hover(node, None);
        }
    }

    /// Handle a mouse wheel event.
    #[cfg(winit)]
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
    #[cfg(winit)]
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
            .mouse_grab
            .as_ref()
            .map(|g| g.button == button)
            .unwrap_or(false)
        {
            if let Some((id, event)) = self.remove_mouse_grab(true) {
                self.send_event(node.re(), id, event);
            }
        }

        if state == ElementState::Pressed {
            if let Some(start_id) = self.mouse.hover.clone() {
                // No mouse grab but have a hover target
                if self.config.event().mouse_nav_focus() {
                    if let Some(id) = self.nav_next(node.re(), Some(&start_id), NavAdvance::None) {
                        self.set_nav_focus(id, FocusSource::Pointer);
                    }
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
