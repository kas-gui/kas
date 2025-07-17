// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: IsUsed and Scroll types

use crate::geom::{Offset, Rect};

pub use IsUsed::{Unused, Used};

use super::components::KineticStart;

/// Return type of event-handling methods
///
/// This type is convertible to/from `bool` and supports the expected bit-wise
/// OR operator (`a | b`, `*a |= b`).
///
/// The type also implements negation with output type `bool`, thus allowing
/// `if is_used.into() { ... }` and `if !is_used { ... }`. An implementation of
/// `Deref` would be preferred, but the trait can only output a reference.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum IsUsed {
    /// Event was unused
    ///
    /// Unused events may be used by a parent/ancestor widget or passed to
    /// another handler until used.
    Unused,
    /// Event is used, no other result
    Used,
}

impl From<bool> for IsUsed {
    fn from(is_used: bool) -> Self {
        match is_used {
            false => Self::Unused,
            true => Self::Used,
        }
    }
}

impl From<IsUsed> for bool {
    fn from(is_used: IsUsed) -> bool {
        is_used == Used
    }
}

impl std::ops::BitOr for IsUsed {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Unused, Unused) => Unused,
            _ => Used,
        }
    }
}
impl std::ops::BitOrAssign for IsUsed {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::Not for IsUsed {
    type Output = bool;
    #[inline]
    fn not(self) -> bool {
        self != Used
    }
}

/// Request to / notification of scrolling from a child
///
/// See: [`EventCx::set_scroll`](super::EventCx::set_scroll).
#[derive(Clone, Debug, Default, PartialEq)]
#[must_use]
pub enum Scroll {
    /// No scrolling
    #[default]
    None,
    /// Child has scrolled; no further scrolling needed
    ///
    /// External scroll bars use this as a notification to update self.
    Scrolled,
    /// Pan region by the given offset
    ///
    /// This may be returned to scroll the closest scrollable ancestor region.
    /// This region should attempt to scroll self by this offset, then, if all
    /// the offset was used, return `Scroll::Scrolled`, otherwise return
    /// `Scroll::Offset(delta)` with the unused offset `delta`.
    ///
    /// With the usual scroll offset conventions, this delta must be subtracted
    /// from the scroll offset.
    Offset(Offset),
    /// Start kinetic scrolling
    Kinetic(KineticStart),
    /// Focus the given rect
    ///
    /// This is specified in the child's coordinate space. It is assumed that
    /// any parent with non-zero translation will intercept this value and
    /// either consume or translate it.
    Rect(Rect),
}
