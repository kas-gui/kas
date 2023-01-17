// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window trait and identifier

use super::{Icon, Widget};
use crate::event::EventMgr;
use std::num::NonZeroU32;

/// Identifier for a window or pop-up
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    /// Construct a [`WindowId`]
    ///
    /// Only for use by the shell!
    pub(crate) fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

/// Functionality required by a window
pub trait Window: Widget {
    /// Get the window title
    fn title(&self) -> &str;

    /// Get the window icon, if any
    ///
    /// Default: `None`
    #[inline]
    fn icon(&self) -> Option<Icon> {
        None
    }

    /// Whether to limit the maximum size of a window
    ///
    /// All widgets' size rules allow calculation of two sizes: the minimum
    /// size and the ideal size. Windows are initially sized to the ideal size.
    /// This option controls whether the window size is restricted by the
    /// calculated minimum size and by the ideal size.
    ///
    /// Return value is `(restrict_min, restrict_max)`. Suggested is to use
    /// `(true, true)` for simple dialog boxes and `(true, false)` for complex
    /// windows.
    ///
    /// Default: `(true, false)`
    #[inline]
    fn restrict_dimensions(&self) -> (bool, bool) {
        (true, false)
    }

    /// Handle closure of self
    ///
    /// This allows for actions on destruction.
    ///
    /// Default: do nothing.
    fn handle_closure(&mut self, mgr: &mut EventMgr) {
        let _ = mgr;
    }
}
