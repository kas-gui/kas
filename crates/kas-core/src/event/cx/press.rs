// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[allow(unused)] use super::{Event, EventState}; // for doc-links
use super::{EventCx, GrabMode, IsUsed, MouseGrab, TouchGrab};
use crate::event::cx::GrabDetails;
use crate::event::{CursorIcon, MouseButton, Unused, Used};
use crate::geom::Coord;
use crate::{Action, Id};

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
    pub id: Option<Id>,
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
    pub fn grab(&self, id: Id, mode: GrabMode) -> GrabBuilder {
        GrabBuilder {
            id,
            source: self.source,
            coord: self.coord,
            mode,
            cursor: None,
        }
    }
}

/// Bulider pattern (see [`Press::grab`])
///
/// Conclude by calling [`Self::with_cx`].
#[must_use]
pub struct GrabBuilder {
    id: Id,
    source: PressSource,
    coord: Coord,
    mode: GrabMode,
    cursor: Option<CursorIcon>,
}

impl GrabBuilder {
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
    ///
    /// In case of an existing grab for the same [`source`](Press::source),
    /// - If the [`Id`] differs this fails (returns [`Unused`])
    /// - If the [`MouseButton`] differs this fails (technically this is a
    ///   different `source`, but simultaneous grabs of multiple mouse buttons
    ///   are not supported).
    /// - If one grab is a [pan](GrabMode::is_pan) and the other is not, this fails
    /// - [`GrabMode::Click`] may be upgraded to [`GrabMode::Grab`]
    /// - Changing from one pan mode to another is an error
    /// - Mouse button repetitions may be increased; decreasing is an error
    /// - A [`CursorIcon`] may be set
    /// - The depress target is re-set to the grabbing widget
    ///
    /// Note: error conditions are only checked in debug builds. These cases
    /// may need revision.
    pub fn with_cx(self, cx: &mut EventCx) -> IsUsed {
        let GrabBuilder {
            id,
            source,
            coord,
            mode,
            cursor,
        } = self;
        log::trace!(target: "kas_core::event", "grab_press: start_id={id}, source={source:?}");
        match source {
            PressSource::Mouse(button, repetitions) => {
                let details = match mode {
                    GrabMode::Click => GrabDetails::Click,
                    GrabMode::Grab => GrabDetails::Grab,
                    mode => {
                        assert!(mode.is_pan());
                        let g = cx.set_pan_on(id.clone(), mode, false, coord);
                        GrabDetails::Pan(g)
                    }
                };
                if let Some(ref mut grab) = cx.mouse_grab {
                    if grab.start_id != id
                        || grab.button != button
                        || grab.details.is_pan() != mode.is_pan()
                        || grab.cancel
                    {
                        return Unused;
                    }

                    debug_assert!(repetitions >= grab.repetitions);
                    grab.repetitions = repetitions;
                    grab.depress = Some(id.clone());
                    grab.details = details;
                } else {
                    cx.mouse_grab = Some(MouseGrab {
                        button,
                        repetitions,
                        start_id: id.clone(),
                        depress: Some(id.clone()),
                        details,
                        cancel: false,
                    });
                }
                if let Some(icon) = cursor {
                    cx.window.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if let Some(grab) = cx.get_touch(touch_id) {
                    if grab.mode.is_pan() != mode.is_pan() || grab.cancel {
                        return Unused;
                    }

                    grab.depress = Some(id.clone());
                    grab.cur_id = Some(id.clone());
                    grab.last_move = coord;
                    grab.coord = coord;
                    grab.mode = grab.mode.max(mode);
                } else {
                    let mut pan_grab = (u16::MAX, 0);
                    if mode.is_pan() {
                        pan_grab = cx.set_pan_on(id.clone(), mode, true, coord);
                    }
                    cx.touch_grab.push(TouchGrab {
                        id: touch_id,
                        start_id: id.clone(),
                        depress: Some(id.clone()),
                        cur_id: Some(id.clone()),
                        last_move: coord,
                        coord,
                        mode,
                        pan_grab,
                        cancel: false,
                    });
                }
            }
        }

        cx.action(id, Action::REDRAW);
        Used
    }
}
