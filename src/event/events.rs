// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

#[allow(unused)]
use super::Manager; // for doc-links
use super::{MouseButton, UpdateHandle};

use crate::geom::{Coord, DVec2};
use crate::WidgetId;

/// High-level events addressed to a widget by [`WidgetId`]
#[derive(Clone, Debug)]
pub enum Action {
    /// Widget activation, for example clicking a button or toggling a check-box
    Activate,
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

/// Low-level events addressed to a widget by [`WidgetId`] or coordinate.
#[derive(Clone, Debug)]
pub enum Event {
    Action(Action),
    /// A mouse button was pressed or touch event started
    PressStart {
        source: PressSource,
        coord: Coord,
    },
    /// Movement of mouse or a touch press
    ///
    /// Received only given a [press grab](super::Manager::request_grab).
    PressMove {
        source: PressSource,
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

/// Type used by [`Action::Scroll`]
#[derive(Clone, Copy, Debug)]
pub enum ScrollDelta {
    /// Scroll a given number of lines
    LineDelta(f32, f32),
    /// Scroll a given number of pixels
    PixelDelta(Coord),
}
