// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[allow(unused)] use super::{Event, EventState}; // for doc-links
use super::{EventCx, GrabMode, MouseGrab, Pending, Response, TouchGrab};
use crate::event::{CursorIcon, MouseButton, Used};
use crate::geom::{Coord, Offset};
use crate::{Action, WidgetId};

/// Source of `EventChild::Press`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressSource {
    /// A mouse click
    ///
    /// Arguments: `button, repeats`.
    ///
    /// The `repeats` argument is used for double-clicks and similar. For a
    /// single-click, `repeats == 1`; for a double-click it is 2, for a
    /// triple-click it is 3, and so on (without upper limit).
    ///
    /// For `PressMove` and `PressEnd` events delivered with a mouse-grab,
    /// both arguments are copied from the initiating `PressStart` event.
    /// For `CursorMove` delivered without a grab (only possible with pop-ups)
    /// a fake `button` value is used and `repeats == 0`.
    Mouse(MouseButton, u32),
    /// A touch event (with given `id`)
    Touch(u64),
}

impl PressSource {
    /// Returns true if this represents the left mouse button or a touch event
    #[inline]
    pub fn is_primary(self) -> bool {
        match self {
            PressSource::Mouse(button, _) => button == MouseButton::Left,
            PressSource::Touch(_) => true,
        }
    }

    /// Returns true if this represents the right mouse button
    #[inline]
    pub fn is_secondary(self) -> bool {
        matches!(self, PressSource::Mouse(MouseButton::Right, _))
    }

    /// Returns true if this represents the middle mouse button
    #[inline]
    pub fn is_tertiary(self) -> bool {
        matches!(self, PressSource::Mouse(MouseButton::Middle, _))
    }

    /// Returns true if this represents a touch event
    #[inline]
    pub fn is_touch(self) -> bool {
        matches!(self, PressSource::Touch(_))
    }

    /// The `repetitions` value
    ///
    /// This is 1 for a single-click and all touch events, 2 for a double-click,
    /// 3 for a triple-click, etc. For `CursorMove` without a grab this is 0.
    #[inline]
    pub fn repetitions(self) -> u32 {
        match self {
            PressSource::Mouse(_, repetitions) => repetitions,
            PressSource::Touch(_) => 1,
        }
    }
}

/// Details of press events
#[crate::autoimpl(Deref using self.source)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Press {
    /// Source
    pub source: PressSource,
    /// Identifier of current widget
    pub id: Option<WidgetId>,
    /// Current coordinate
    pub coord: Coord,
}

impl Press {
    /// Grab pan/move/press-end events for widget `id`
    ///
    /// There are three types of grab ([`GrabMode`]):
    ///
    /// -   `Click`: send the corresponding [`Event::PressEnd`] only
    /// -   `Grab` (the default): send [`Event::PressMove`] and [`Event::PressEnd`]
    /// -   Pan modes: send [`Event::Pan`] on motion.
    ///     Note: this is most useful when grabbing multiple touch events.
    ///
    /// Only a single mouse grab is allowed at any one time; requesting a
    /// second will cancel the first (sending [`Event::PressEnd`] with
    /// `success: false`).
    ///
    /// [`EventState::is_depressed`] will return true for the grabbing widget.
    /// Call [`EventState::set_grab_depress`] on `PressMove` to update the
    /// grab's depress target. (This is done automatically for
    /// [`GrabMode::Click`], and ends automatically when the grab ends.)
    ///
    /// This method uses the builder pattern. On completion, [`Used`]
    /// is returned. It is expected that the requested press/pan events are all
    /// "used" ([`Used`]).
    #[inline]
    pub fn grab(&self, id: WidgetId) -> GrabBuilder {
        GrabBuilder {
            id,
            source: self.source,
            coord: self.coord,
            mode: GrabMode::Grab,
            cursor: None,
        }
    }
}

/// Bulider pattern (see [`Press::grab`])
///
/// Conclude by calling [`Self::with_cx`].
#[must_use]
pub struct GrabBuilder {
    id: WidgetId,
    source: PressSource,
    coord: Coord,
    mode: GrabMode,
    cursor: Option<CursorIcon>,
}

impl GrabBuilder {
    /// Set grab mode (default: [`GrabMode::Grab`])
    #[inline]
    pub fn with_mode(mut self, mode: GrabMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set cursor icon (default: do not set)
    #[inline]
    pub fn with_icon(self, icon: CursorIcon) -> Self {
        self.with_opt_icon(Some(icon))
    }

    /// Optionally set cursor icon (default: do not set)
    #[inline]
    pub fn with_opt_icon(mut self, icon: Option<CursorIcon>) -> Self {
        self.cursor = icon;
        self
    }

    /// Complete the grab, providing the [`EventCx`]
    pub fn with_cx(self, cx: &mut EventCx) -> Response {
        let GrabBuilder {
            id,
            source,
            coord,
            mode,
            cursor,
        } = self;
        log::trace!(target: "kas_core::event", "grab_press: start_id={id}, source={source:?}");
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button, repetitions) => {
                if let Some((id, event)) = cx.remove_mouse_grab(false) {
                    cx.pending.push_back(Pending::Send(id, event));
                }
                if mode.is_pan() {
                    pan_grab = cx.set_pan_on(id.clone(), mode, false, coord);
                }
                cx.mouse_grab = Some(MouseGrab {
                    button,
                    repetitions,
                    start_id: id.clone(),
                    cur_id: Some(id.clone()),
                    depress: Some(id),
                    mode,
                    pan_grab,
                    coord,
                    delta: Offset::ZERO,
                });
                if let Some(icon) = cursor {
                    cx.shell.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if cx.remove_touch(touch_id).is_some() {
                    #[cfg(debug_assertions)]
                    log::error!(target: "kas_core::event", "grab_press: touch_id conflict!");
                }
                if mode.is_pan() {
                    pan_grab = cx.set_pan_on(id.clone(), mode, true, coord);
                }
                cx.touch_grab.push(TouchGrab {
                    id: touch_id,
                    start_id: id.clone(),
                    depress: Some(id.clone()),
                    cur_id: Some(id),
                    last_move: coord,
                    coord,
                    mode,
                    pan_grab,
                });
            }
        }

        cx.send_action(Action::REDRAW);
        Used
    }
}
