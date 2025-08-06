// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: mouse events

use super::{GrabMode, Press, PressSource, velocity};
use crate::event::{Event, EventCx, EventState, FocusSource, PressStart, ScrollDelta, TimerHandle};
use crate::geom::{Affine, Coord, DVec2};
use crate::window::Window;
use crate::window::WindowErased;
use crate::{Action, Id, NavAdvance, Node, TileExt, Widget};
use cast::{Cast, CastApprox, Conv, ConvApprox};
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

pub(crate) struct Mouse {
    pub(super) over: Option<Id>, // widget under the mouse
    pub(super) icon: CursorIcon,
    old_icon: CursorIcon,
    last_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    last_pin: Option<(Id, DVec2)>,
    pub(super) grab: Option<MouseGrab>,
    tooltip_source: Option<Id>,
    last_position: DVec2,
    pub(super) samples: velocity::Samples,
}

impl Default for Mouse {
    fn default() -> Self {
        Mouse {
            over: None,
            icon: CursorIcon::Default,
            old_icon: CursorIcon::Default,
            last_coord: Coord::ZERO,
            last_click_button: FAKE_MOUSE_BUTTON,
            last_click_repetitions: 0,
            last_click_timeout: Instant::now(),
            last_pin: None,
            grab: None,
            tooltip_source: None,
            last_position: DVec2::ZERO,
            samples: Default::default(),
        }
    }
}

impl Mouse {
    pub(crate) const TIMER_HOVER: TimerHandle = TimerHandle::new(1 << 59, false);

    /// Clear all focus and grabs on `target`
    pub(in crate::event::cx) fn cancel_event_focus(&mut self, target: &Id) {
        if let Some(grab) = self.grab.as_mut()
            && grab.start_id == target
        {
            grab.cancel = true;
        }
    }

    /// Call on frame to detect change in mouse cursor icon
    pub(in crate::event::cx) fn update_cursor_icon(&mut self) -> Option<CursorIcon> {
        let mut icon = None;
        if self.icon != self.old_icon && self.grab.is_none() {
            icon = Some(self.icon);
        }
        self.old_icon = self.icon;
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

    /// Identifier of widget under the mouse
    pub(crate) fn over_id(&self) -> Option<Id> {
        self.over.clone()
    }

    fn update_grab(&mut self) -> (bool, bool) {
        let (mut cancel, mut redraw) = (false, false);
        if let Some(grab) = self.grab.as_mut() {
            cancel = grab.cancel;
            if let GrabDetails::Click = grab.details {
                let over = self.over.as_ref();
                if grab.start_id == over {
                    if grab.depress.as_ref() != over {
                        grab.depress = over.cloned();
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
    pub(in crate::event::cx) fn start_grab(
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
                // coord may have been offset by a scroll region; we must keep
                // that but should try to preserve fractional precision.
                let position = DVec2::conv(coord) + self.last_position.fract();

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

    pub(in crate::event::cx) fn tooltip_popup_close(&mut self, id: &Id) {
        if self.tooltip_source.as_ref() == Some(id) {
            self.tooltip_source = None;
        }
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
    // Clear old `over` id, set new `over`, send events.
    // If there is a popup, only permit descendants of that.
    fn set_over(&mut self, mut window: Node<'_>, w_id: Option<Id>) {
        if self.mouse.over != w_id {
            log::trace!("set_over: w_id={w_id:?}");
            self.mouse.icon = Default::default();
            if let Some(id) = self.mouse.over.take() {
                self.send_event(window.re(), id, Event::MouseOver(false));
            }
            self.mouse.over = w_id.clone();
            let delay = self.config().event().hover_delay();
            self.request_timer(window.id(), Mouse::TIMER_HOVER, delay);

            if let Some(id) = w_id {
                self.send_event(window, id, Event::MouseOver(true));
            }
        }
    }

    // Clears mouse grab and pan grab, resets cursor and redraws
    fn remove_mouse_grab(&mut self, window: Node<'_>, success: bool) {
        let mut to_send = None;
        let last_pin;
        let redraw;
        if let Some(grab) = self.mouse.grab.as_ref() {
            log::trace!(
                "remove_mouse_grab: start_id={}, success={success}",
                grab.start_id
            );
            self.window.set_cursor_icon(self.mouse.icon);
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
                    source: PressSource::mouse(grab.button, grab.repetitions),
                    id: self.mouse.over.clone(),
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
            self.send_event(window, id, event);
        }
        self.mouse.last_pin = last_pin;
        self.opt_action(redraw, Action::REDRAW);

        self.mouse.grab = None;
    }

    pub(in crate::event::cx) fn mouse_handle_pending<A>(&mut self, win: &mut Window<A>, data: &A) {
        let (cancel, redraw) = self.mouse.update_grab();
        if cancel {
            self.remove_mouse_grab(win.as_node(data), false);
        }

        if redraw {
            self.action |= Action::REDRAW;
        }

        if self.action.contains(Action::REGION_MOVED) {
            let over = win.try_probe(self.mouse.last_coord);
            self.set_over(win.as_node(data), over);
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

        let id = win.try_probe(coord);
        self.tooltip_motion(win, &id);
        self.handle_cursor_moved_(id, win.as_node(data), coord, position);
    }

    pub(in crate::event::cx) fn handle_cursor_moved_(
        &mut self,
        id: Option<Id>,
        mut window: Node<'_>,
        coord: Coord,
        position: DVec2,
    ) {
        self.set_over(window.re(), id.clone());

        if let Some(grab) = self.mouse.grab.as_mut() {
            match &mut grab.details {
                GrabDetails::Click => (),
                GrabDetails::Grab => {
                    let target = grab.start_id.clone();
                    let press = Press {
                        source: PressSource::mouse(grab.button, grab.repetitions),
                        id,
                        coord,
                    };
                    let delta = coord - self.mouse.last_coord;
                    let event = Event::PressMove { press, delta };
                    self.send_event(window.re(), target, event);
                }
                GrabDetails::Pan(details) => {
                    details.c1 = position;
                    details.moved = true;
                    self.need_frame_update = true;
                }
            }
        } else if let Some(popup_id) = self
            .popups
            .last()
            .filter(|popup| popup.is_sized)
            .map(|state| state.desc.id.clone())
        {
            let press = Press {
                source: PressSource::mouse(FAKE_MOUSE_BUTTON, 0),
                id,
                coord,
            };
            let event = Event::CursorMove { press };
            self.send_event(window, popup_id, event);
        } else {
            // We don't forward move events without a grab
        }

        self.mouse.last_coord = coord;
    }

    /// Handle mouse cursor entering the app.
    #[inline]
    pub(in crate::event::cx) fn handle_cursor_entered(&mut self) {}

    /// Handle mouse cursor leaving the app.
    pub(in crate::event::cx) fn handle_cursor_left(&mut self, window: Node<'_>) {
        self.mouse.last_click_button = FAKE_MOUSE_BUTTON;

        if self.mouse.grab.is_none() {
            // If there's a mouse grab, we will continue to receive
            // coordinates; if not, set a fake coordinate off the window
            self.mouse.last_coord = Coord(-1, -1);
            self.set_over(window, None);
        }
    }

    /// Handle a mouse wheel event.
    pub(in crate::event::cx) fn handle_mouse_wheel(
        &mut self,
        window: Node<'_>,
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
        if let Some(id) = self.mouse.over.clone() {
            self.send_event(window, id, event);
        }
    }

    /// Handle a mouse click / release.
    pub(in crate::event::cx) fn handle_mouse_input(
        &mut self,
        mut window: Node<'_>,
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
            self.remove_mouse_grab(window.re(), true);
        }

        if state == ElementState::Pressed {
            let start_id = self.mouse.over.clone();
            self.close_non_ancestors_of(start_id.as_ref());

            if let Some(id) = start_id {
                // No mouse grab but have a widget under the mouse
                if matches!(self.mouse.last_pin.as_ref(), Some((pin_id, _)) if *pin_id != id) {
                    self.mouse.last_pin = None;
                }

                if self.config.event().mouse_nav_focus()
                    && let Some(id) = self.nav_next(window.re(), Some(&id), NavAdvance::None)
                {
                    self.set_nav_focus(id, FocusSource::Pointer);
                }

                let source = PressSource::mouse(button, self.mouse.last_click_repetitions);
                let press = PressStart {
                    source,
                    id: Some(id.clone()),
                    coord: self.mouse.last_coord,
                };
                let event = Event::PressStart(press);
                self.send_event(window, id, event);
            }
        }
    }

    /// Call on TIMER_HOVER expiry
    pub(crate) fn hover_timer_expiry(&mut self, win: &mut dyn WindowErased) {
        match (self.mouse.over_id(), &self.mouse.tooltip_source) {
            (None, None) => (),
            (None, Some(_)) => {
                win.close_tooltip(self);
                self.mouse.tooltip_source = None;
            }
            (Some(id), Some(source)) if id == source => (),
            (Some(id), _) => {
                if let Some(text) = win.as_tile().find_tile(&id).and_then(|tile| tile.tooltip()) {
                    win.show_tooltip(self, id.clone(), text.to_string());
                    self.mouse.tooltip_source = Some(id);
                } else {
                    win.close_tooltip(self);
                    self.mouse.tooltip_source = None;
                }
            }
        }
    }

    fn tooltip_motion(&mut self, win: &mut dyn WindowErased, id: &Option<Id>) {
        match &mut self.mouse.tooltip_source {
            Some(source) if *source != id => {
                if let Some(id) = id.as_ref()
                    && let Some(text) = win.as_tile().find_tile(id).and_then(|tile| tile.tooltip())
                {
                    win.show_tooltip(self, id.clone(), text.to_string());
                    self.mouse.tooltip_source = Some(id.clone());
                }
            }
            _ => (),
        }
    }
}
