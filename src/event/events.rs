// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[allow(unused)]
use super::Manager; // for doc-links
use super::{MouseButton, UpdateHandle, VirtualKeyCode};

use crate::geom::{Coord, DVec2};
use crate::WidgetId;

/// Events addressed to a widget
#[derive(Clone, Debug)]
pub enum Event {
    /// Widget activation, for example clicking a button or toggling a check-box
    Activate,
    /// Navigation key input
    ///
    /// This is received only when the widget has key-navigation focus. Note
    /// that [`Event::Activate`] is also effectively a navigation key.
    NavKey(NavKey),
    /// Widget lost keyboard input focus
    LostCharFocus,
    /// Widget receives a character of text input
    ReceivedCharacter(char),
    /// A mouse or touchpad scroll event
    Scroll(ScrollDelta),
    /// A mouse or touch-screen move/zoom/rotate event
    ///
    /// Mouse-grabs generate translation (`delta` component) only. Touch grabs
    /// optionally also generate rotation and scaling components.
    ///
    /// In general, a point `p` on the screen should be transformed as follows:
    /// ```
    /// # use kas::geom::{Coord, DVec2};
    /// # let (alpha, delta) = (DVec2::ZERO, DVec2::ZERO);
    /// # let mut p = Coord::ZERO;
    /// // Works for Coord type; for DVec2 type-conversions are unnecessary:
    /// p = (alpha.complex_mul(p.into()) + delta).into();
    /// ```
    ///
    /// When it is known that there is no rotational component, one can use a
    /// simpler transformation: `alpha.0 * p + delta`. When there is also no
    /// scaling component, we just have a translation: `p + delta`.
    /// Note however that if events are generated with rotation and/or scaling
    /// components, these simplifications are invalid.
    ///
    /// Two such transforms may be combined as follows:
    /// ```
    /// # use kas::geom::DVec2;
    /// # let (alpha1, delta1) = (DVec2::ZERO, DVec2::ZERO);
    /// # let (alpha2, delta2) = (DVec2::ZERO, DVec2::ZERO);
    /// let alpha = alpha2.complex_mul(alpha1);
    /// let delta = alpha2.complex_mul(delta1) + delta2;
    /// ```
    /// If instead one uses a transform to map screen-space to world-space,
    /// this transform should be adjusted as follows:
    /// ```
    /// # use kas::geom::DVec2;
    /// # let (alpha, delta) = (DVec2::ZERO, DVec2::ZERO);
    /// # let (mut world_alpha, mut world_delta) = (DVec2::ZERO, DVec2::ZERO);
    /// world_alpha = world_alpha.complex_div(alpha.into());
    /// world_delta = world_delta - world_alpha.complex_mul(delta.into());
    /// ```
    ///
    /// Those familiar with complex numbers may recognise that
    /// `alpha = a * e^{i*t}` where `a` is the scale component and `t` is the
    /// angle of rotation. Calculate these components as follows:
    /// ```
    /// # use kas::geom::DVec2;
    /// # let alpha = DVec2::ZERO;
    /// let a = (alpha.0 * alpha.0 + alpha.1 * alpha.1).sqrt();
    /// let t = (alpha.1).atan2(alpha.0);
    /// ```
    Pan {
        /// Rotation and scale component
        alpha: DVec2,
        /// Translation component
        delta: DVec2,
    },
    /// A mouse button was pressed or touch event started
    PressStart { source: PressSource, coord: Coord },
    /// Movement of mouse or a touch press
    ///
    /// Received only given a [press grab](super::Manager::request_grab).
    PressMove {
        source: PressSource,
        cur_id: Option<WidgetId>,
        coord: Coord,
        delta: Coord,
    },
    /// End of a click/touch press
    ///
    /// Received only given a [press grab](super::Manager::request_grab).
    ///
    /// When `end_id == None`, this is a "cancelled press": the end of the press
    /// is outside the application window.
    PressEnd {
        source: PressSource,
        end_id: Option<WidgetId>,
        coord: Coord,
    },
    /// Update from a timer
    ///
    /// This event is received after requesting timed wake-up(s)
    /// (see [`Manager::update_on_timer`]).
    TimerUpdate,
    /// Update triggerred via an [`UpdateHandle`]
    ///
    /// This event may be received after registering an [`UpdateHandle`] via
    /// [`Manager::update_on_handle`].
    ///
    /// A user-defined payload is passed. Interpretation of this payload is
    /// user-defined and unfortunately not type safe.
    HandleUpdate { handle: UpdateHandle, payload: u64 },
}

/// Navigation key ([`Event::NavKey`])
///
/// The purpose of this enum (instead of sending the active widget a
/// [`VirtualKeyCode`]) is consistent behaviour: these "navigation keys" will
/// always be sent to the widget highlighted for keyboard navigation, if active,
/// while alpha-numeric keys will always be available for accelerator keys
/// (when a character input grab is not present). Additionally, this allows
/// uniform behaviour with regards to num-pad keys.
// TODO: possible expansion candidates: Cut/Copy/Paste,
// Space, Enter, ScrollLock, Pause, Insert, Delete, Backspace
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavKey {
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
}

impl NavKey {
    /// Try constructing from a [`VirtualKeyCode`]
    pub fn new(vkey: VirtualKeyCode) -> Option<Self> {
        use VirtualKeyCode::*;
        Some(match vkey {
            Home => NavKey::Home,
            End => NavKey::End,
            PageDown => NavKey::PageDown,
            PageUp => NavKey::PageUp,
            Left => NavKey::Left,
            Up => NavKey::Up,
            Right => NavKey::Right,
            Down => NavKey::Down,
            _ => return None,
        })
    }
}

/// Source of `EventChild::Press`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressSource {
    /// A mouse click
    Mouse(MouseButton),
    /// A touch event (with given `id`)
    Touch(u64),
}

impl PressSource {
    /// Returns true if this represents the left mouse button or a touch event
    #[inline]
    pub fn is_primary(self) -> bool {
        match self {
            PressSource::Mouse(button) => button == MouseButton::Left,
            PressSource::Touch(_) => true,
        }
    }
}

/// Type used by [`Event::Scroll`]
#[derive(Clone, Copy, Debug)]
pub enum ScrollDelta {
    /// Scroll a given number of lines
    LineDelta(f32, f32),
    /// Scroll a given number of pixels
    PixelDelta(Coord),
}
