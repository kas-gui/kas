// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: IsUsed type

use crate::geom::{Offset, Rect};

/// IsUsed from [`Events::handle_event`]
///
/// [`Events::handle_event`]: crate::Events::handle_event
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

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl IsUsed {
    /// True if variant is `Used`
    #[inline]
    pub fn is_used(&self) -> bool {
        matches!(self, IsUsed::Used)
    }

    /// True if variant is `Unused`
    #[inline]
    pub fn is_unused(&self) -> bool {
        matches!(self, IsUsed::Unused)
    }
}

impl std::ops::BitOr for IsUsed {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        use IsUsed::{Unused, Used};
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

/// Request to / notification of scrolling from a child
///
/// See: [`EventCx::set_scroll`](super::EventCx::set_scroll).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
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
    /// Focus the given rect
    Rect(Rect),
}
