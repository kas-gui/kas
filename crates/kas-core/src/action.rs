// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Action types

/// Action: widget has moved/opened/closed
///
/// This action indicates that the following should happen:
///
/// -   Re-probe which widget is under the mouse / any touch instance / any
///     other location picker since widgets may have moved
/// -   Redraw the window
#[must_use]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionMoved;

/// Action: widget must be resized
///
/// This type implies that either a local or full-window resize is required.
#[must_use]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionResize;

/// Action: content must be redrawn
#[must_use]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionRedraw;

/// Action: close window
#[must_use]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ActionClose;

bitflags! {
    /// Action: configuration data updates must be applied
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct ConfigAction: u32 {
        /// Event configuration data must be updated
        const EVENT = 1 << 0;
        /// Theme configuration data must be updated
        const THEME = 1 << 10;
        /// The theme must be switched
        const THEME_SWITCH = 1 << 12;
    }
}

/// Actions which may affect a window
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[must_use]
pub enum WindowActions {
    /// No action required
    #[default]
    None,
    /// Redraw the active window
    Redraw,
    /// Re-probe content under the mouse and redraw
    ///
    /// This implies that content moved on the screen (e.g. content scrolled or
    /// was internally resized).
    Moved,
    /// Resize, re-probe and redraw
    Resize,
    /// Close the window
    Close,
}
impl From<ActionRedraw> for WindowActions {
    #[inline]
    fn from(_: ActionRedraw) -> Self {
        WindowActions::Redraw
    }
}
impl From<Option<ActionRedraw>> for WindowActions {
    #[inline]
    fn from(opt: Option<ActionRedraw>) -> Self {
        match opt {
            None => WindowActions::None,
            Some(ActionRedraw) => WindowActions::Redraw,
        }
    }
}
impl From<ActionMoved> for WindowActions {
    #[inline]
    fn from(_: ActionMoved) -> Self {
        WindowActions::Moved
    }
}
impl From<Option<ActionMoved>> for WindowActions {
    #[inline]
    fn from(opt: Option<ActionMoved>) -> Self {
        match opt {
            None => WindowActions::None,
            Some(ActionMoved) => WindowActions::Moved,
        }
    }
}
impl From<ActionResize> for WindowActions {
    #[inline]
    fn from(_: ActionResize) -> Self {
        WindowActions::Resize
    }
}
impl From<Option<ActionResize>> for WindowActions {
    #[inline]
    fn from(opt: Option<ActionResize>) -> Self {
        match opt {
            None => WindowActions::None,
            Some(ActionResize) => WindowActions::Resize,
        }
    }
}
impl From<ActionClose> for WindowActions {
    #[inline]
    fn from(_: ActionClose) -> Self {
        WindowActions::Close
    }
}
impl WindowActions {
    pub(crate) fn redraw(self) -> bool {
        self >= WindowActions::Redraw
    }
    pub(crate) fn resize(self) -> bool {
        self == WindowActions::Resize
    }
    pub(crate) fn is_close(self) -> bool {
        matches!(self, WindowActions::Close)
    }

    pub(crate) fn upgrade(&mut self, rhs: impl Into<Self>) {
        let rhs = rhs.into();
        if rhs > *self {
            *self = rhs;
        }
    }
}
