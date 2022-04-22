// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling: Response type

use crate::geom::{Offset, Rect};

/// Response type from [`Handler::handle`].
///
/// This type wraps [`Handler::Msg`] allowing both custom messages and toolkit
/// messages.
///
/// [`Handler::handle`]: super::Handler::handle
/// [`Handler::Msg`]: super::Handler::Msg
#[derive(Copy, Clone, Debug)]
#[must_use]
pub enum Response {
    /// Event was unused
    ///
    /// Unused events may be used by a parent/ancestor widget or passed to
    /// another handler until used.
    Unused,
    /// Event is used, no other result
    ///
    /// All variants besides `Unused` indicate that the event was used. This
    /// variant is used when no further action happens.
    Used,
    /// Pan scrollable regions by the given delta
    ///
    /// This may be returned to scroll the closest scrollable ancestor region.
    /// This region should attempt to scroll self by this offset, then, if all
    /// the offset was used, return `Response::Scrolled`, otherwise return
    /// `Response::Pan(d)` with the unused offset `d`.
    ///
    /// With the usual scroll offset conventions, this delta must be subtracted
    /// from the scroll offset.
    Pan(Offset),
    /// Notify that an inner region scrolled
    Scrolled,
    /// (Keyboard) focus has changed
    ///
    /// This region (in the child's coordinate space) should be made visible.
    Focus(Rect),
    /// Widget wishes to be selected (or have selection status toggled)
    Select,
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl Response {
    /// True if variant is `Used`
    #[inline]
    pub fn is_used(&self) -> bool {
        matches!(self, Response::Used)
    }

    /// True if variant is `Unused`
    #[inline]
    pub fn is_unused(&self) -> bool {
        matches!(self, Response::Unused)
    }
}
