// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: mouse and touch events

mod mouse;
mod touch;
pub(crate) mod velocity;

use std::mem::transmute;

#[allow(unused)] use super::{Event, EventState}; // for doc-links
use super::{EventCx, IsUsed};
#[allow(unused)] use crate::Events; // for doc-links
use crate::event::{CursorIcon, MouseButton, Unused, Used};
use crate::geom::{Coord, Vec2};
use crate::{Action, Id};
pub(crate) use mouse::Mouse;
pub(crate) use touch::Touch;

/// Controls the types of events delivered by [`Press::grab`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrabMode {
    /// Deliver [`Event::PressEnd`] only for each grabbed press
    Click,
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each grabbed press
    Grab,
    /// Deliver [`Event::Pan`] events
    ///
    /// Scaling and rotation are optional.
    Pan { scale: bool, rotate: bool },
}

impl GrabMode {
    /// [`GrabMode::Pan`] without scaling or rotation
    pub const PAN_NONE: GrabMode = GrabMode::Pan {
        scale: false,
        rotate: false,
    };
    /// [`GrabMode::Pan`] with scaling only
    pub const PAN_SCALE: GrabMode = GrabMode::Pan {
        scale: true,
        rotate: false,
    };
    /// [`GrabMode::Pan`] with rotation only
    pub const PAN_ROTATE: GrabMode = GrabMode::Pan {
        scale: false,
        rotate: true,
    };
    /// [`GrabMode::Pan`] with scaling and rotation
    pub const PAN_FULL: GrabMode = GrabMode::Pan {
        scale: true,
        rotate: true,
    };

    /// True for "pan" variants
    #[inline]
    pub fn is_pan(self) -> bool {
        matches!(self, GrabMode::Pan { .. })
    }
}

/// Source of a [`Press`] event
///
/// This identifies the source of a click, touch or similar event, including
/// which mouse button is pressed and whether this is a double-click (see
/// [`Self::repetitions`]).
///
/// This may be used to track a click/touch, but note that identifiers may be
/// re-used after the event completes, thus an [`Event::PressStart`] with
/// `PressSource` equal to a prior instance does not indicate that the same
/// finger / mouse was used.
/// Further note: identifying multiple mice is not currently supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PressSource(u64);

impl PressSource {
    const FLAG_TOUCH: u64 = 1 << 63;

    /// Construct a mouse source
    pub(crate) fn mouse(button: MouseButton, repetitions: u32) -> Self {
        let r = (repetitions as u64) << 32;
        debug_assert!(r & Self::FLAG_TOUCH == 0);
        let b: u32 = unsafe { transmute(button) };
        Self(r | b as u64)
    }

    /// Construct a touch source
    pub(crate) fn touch(id: u64) -> Self {
        // Investigation shows that almost all sources use a 32-bit identifier.
        // The only exceptional winit backend is iOS, which uses a pointer.
        assert!(id & Self::FLAG_TOUCH == 0);
        Self(Self::FLAG_TOUCH | id)
    }

    /// Returns true if this represents a mouse event
    #[inline]
    pub fn is_mouse(self) -> bool {
        self.0 & Self::FLAG_TOUCH == 0
    }

    /// Returns true if this represents a touch event
    #[inline]
    pub fn is_touch(self) -> bool {
        self.0 & Self::FLAG_TOUCH != 0
    }

    /// Returns the touch identifier if this represents a touch event
    fn touch_id(self) -> Option<u64> {
        if self.is_touch() {
            Some(self.0 & !Self::FLAG_TOUCH)
        } else {
            None
        }
    }

    /// Identify the mouse button used
    ///
    /// This always returns `Some(_)` for mouse events and `None` for touch
    /// events.
    pub fn mouse_button(self) -> Option<MouseButton> {
        if self.is_mouse() {
            let b = self.0 as u32;
            Some(unsafe { transmute::<u32, MouseButton>(b) })
        } else {
            None
        }
    }

    /// Returns true if this represents the left mouse button or a touch event
    #[inline]
    pub fn is_primary(self) -> bool {
        match self.mouse_button() {
            None => true,
            Some(MouseButton::Left) => true,
            Some(_) => false,
        }
    }

    /// Returns true if this represents the right mouse button
    #[inline]
    pub fn is_secondary(self) -> bool {
        match self.mouse_button() {
            Some(MouseButton::Right) => true,
            None | Some(_) => false,
        }
    }

    /// Returns true if this represents the middle mouse button
    #[inline]
    pub fn is_tertiary(self) -> bool {
        match self.mouse_button() {
            Some(MouseButton::Middle) => true,
            None | Some(_) => false,
        }
    }

    /// The `repetitions` value
    ///
    /// This is 1 for a single-click and all touch events, 2 for a double-click,
    /// 3 for a triple-click, etc. For `CursorMove` without a grab this is 0.
    #[inline]
    pub fn repetitions(self) -> u32 {
        if self.is_mouse() {
            // Top 32 bits is repetitions
            (self.0 >> 32) as u32
        } else {
            // Touch events do not support repetitions.
            1
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
/// Conclude by calling [`Self::complete`].
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
    pub fn complete(self, cx: &mut EventCx) -> IsUsed {
        let GrabBuilder {
            id,
            source,
            coord,
            mode,
            cursor,
        } = self;
        log::trace!(target: "kas_core::event", "grab_press: start_id={id}, source={source:?}");
        let success;
        if let Some(button) = source.mouse_button() {
            success = cx
                .mouse
                .start_grab(button, source.repetitions(), id.clone(), coord, mode);
            if success && let Some(icon) = cursor {
                cx.window.set_cursor_icon(icon);
            }
        } else if let Some(touch_id) = source.touch_id() {
            success = cx.touch.start_grab(touch_id, id.clone(), coord, mode)
        } else {
            unreachable!()
        };

        if success {
            cx.action(id, Action::REDRAW);
            Used
        } else {
            Unused
        }
    }
}

/// Mouse and touch methods
impl EventState {
    /// Check whether the given widget is visually depressed
    pub fn is_depressed(&self, w_id: &Id) -> bool {
        for (_, id) in &self.key_depress {
            if *id == w_id {
                return true;
            }
        }
        if self
            .mouse
            .grab
            .as_ref()
            .map(|grab| *w_id == grab.depress)
            .unwrap_or(false)
        {
            return true;
        }
        for grab in self.touch.touch_grab.iter() {
            if *w_id == grab.depress {
                return true;
            }
        }
        for popup in &self.popups {
            if *w_id == popup.desc.parent {
                return true;
            }
        }
        false
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_under_mouse(&self, w_id: &Id) -> bool {
        self.mouse.grab.is_none() && *w_id == self.mouse.over
    }

    /// Set the cursor icon
    ///
    /// This is normally called from [`Events::handle_mouse_over`]. In other
    /// cases, calling this method may be ineffective. The cursor is
    /// automatically "unset" when the widget is no longer under the mouse.
    ///
    /// See also [`EventCx::set_grab_cursor`]: if a mouse grab
    /// ([`Press::grab`]) is active, its icon takes precedence.
    pub fn set_mouse_over_icon(&mut self, icon: CursorIcon) {
        // Note: this is acted on by EventState::update
        self.mouse.icon = icon;
    }

    /// Set a grab's depress target
    ///
    /// When a grab on mouse or touch input is in effect
    /// ([`Press::grab`]), the widget owning the grab may set itself
    /// or any other widget as *depressed* ("pushed down"). Each grab depresses
    /// at most one widget, thus setting a new depress target clears any
    /// existing target. Initially a grab depresses its owner.
    ///
    /// This effect is purely visual. A widget is depressed when one or more
    /// grabs targets the widget to depress, or when a keyboard binding is used
    /// to activate a widget (for the duration of the key-press).
    ///
    /// Assumption: this method will only be called by handlers of a grab (i.e.
    /// recipients of [`Event::PressStart`] after initiating a successful grab,
    /// [`Event::PressMove`] or [`Event::PressEnd`]).
    ///
    /// Queues a redraw and returns `true` if the depress target changes,
    /// otherwise returns `false`.
    pub fn set_grab_depress(&mut self, source: PressSource, target: Option<Id>) -> bool {
        let mut old = None;
        let mut redraw = false;
        if source.is_mouse() {
            if let Some(grab) = self.mouse.grab.as_mut() {
                redraw = grab.depress != target;
                old = grab.depress.take();
                grab.depress = target.clone();
            }
        } else if let Some(id) = source.touch_id() {
            if let Some(grab) = self.touch.get_touch(id) {
                redraw = grab.depress != target;
                old = grab.depress.take();
                grab.depress = target.clone();
            }
        } else {
            unreachable!()
        }

        if redraw {
            log::trace!(target: "kas_core::event", "set_grab_depress: target={target:?}");
            self.opt_action(old, Action::REDRAW);
            self.opt_action(target, Action::REDRAW);
        }
        redraw
    }

    /// Returns true if there is a mouse or touch grab on `id` or any descendant of `id`
    pub fn any_grab_on(&self, id: &Id) -> bool {
        if self
            .mouse
            .grab
            .as_ref()
            .map(|grab| grab.start_id == id)
            .unwrap_or(false)
        {
            return true;
        }
        self.touch.touch_grab.iter().any(|grab| grab.start_id == id)
    }

    /// Get velocity of the mouse cursor or a touch
    ///
    /// The velocity is calculated at the time this method is called using
    /// existing samples of motion.
    ///
    /// Where the `source` is a mouse this always succeeds.
    /// For touch events this requires an active grab and is not
    /// guaranteed to succeed; currently only a limited number of presses with
    /// mode [`GrabMode::Grab`] are tracked for velocity.
    pub fn press_velocity(&self, source: PressSource) -> Option<Vec2> {
        let evc = self.config().event();
        if source.is_mouse() {
            Some(self.mouse.samples.velocity(evc.kinetic_timeout()))
        } else if let Some(id) = source.touch_id() {
            self.touch.velocity(id, evc)
        } else {
            unreachable!()
        }
    }
}

impl<'a> EventCx<'a> {
    /// Update the mouse cursor used during a grab
    ///
    /// This only succeeds if widget `id` has an active mouse-grab (see
    /// [`Press::grab`]). The cursor will be reset when the mouse-grab
    /// ends.
    pub fn set_grab_cursor(&mut self, id: &Id, icon: CursorIcon) {
        if let Some(ref grab) = self.mouse.grab
            && grab.start_id == *id
        {
            self.window.set_cursor_icon(icon);
        }
    }
}
