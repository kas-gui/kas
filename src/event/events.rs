// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: events

use super::MouseButton;

use crate::geom::Coord;
use crate::WidgetId;

/// Delivery address of an [`Event`]
#[derive(Clone, Copy, Debug)]
pub enum Address {
    Id(WidgetId),
    Coord(Coord),
}

/// High-level events addressed to a widget by [`WidgetId`]
#[derive(Clone, Debug)]
pub enum Action {
    /// Widget activation, for example clicking a button or toggling a check-box
    Activate,
    /// Widget receives a character of text input
    ReceivedCharacter(char),
    /// A mouse or touchpad scroll event
    Scroll(ScrollDelta),
}

/// Low-level events addressed to a widget by [`WidgetId`] or coordinate.
#[derive(Clone, Debug)]
pub enum Event {
    Action(Action),
    Identify,
    /// A mouse button was pressed or touch event started
    PressStart {
        source: PressSource,
        coord: Coord,
    },
    /// Movement of mouse or a
    ///
    /// Received only if a mouse grab is enabled
    PressMove {
        source: PressSource,
        coord: Coord,
        delta: Coord,
    },
    /// End of a click/touch press
    ///
    /// Received if a mouse grab is enabled; otherwise received if on self
    PressEnd {
        source: PressSource,
        start_id: Option<WidgetId>,
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

/// Type used by [`EventChild::Scroll`]
#[derive(Clone, Copy, Debug)]
pub enum ScrollDelta {
    /// Scroll a given number of lines
    LineDelta(f32, f32),
    /// Scroll a given number of pixels
    PixelDelta(Coord),
}
