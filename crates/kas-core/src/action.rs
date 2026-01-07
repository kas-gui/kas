// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Action types

#[allow(unused)]
use crate::event::{ConfigCx, EventCx, EventState};

/// Action: widget has moved/opened/closed
///
/// This action indicates that the following should happen:
///
/// -   Re-probe which widget is under the mouse / any touch instance / any
///     other location picker since widgets may have moved
/// -   Redraw the window
#[must_use]
#[derive(Copy, Clone, Debug, Default)]
pub struct ActionMoved;

/// Action: widget must be resized
///
/// This type implies that either a local or full-window resize is required.
#[must_use]
#[derive(Copy, Clone, Debug, Default)]
pub struct ActionResize;

/// Action: content must be redrawn
#[must_use]
#[derive(Copy, Clone, Debug, Default)]
pub struct ActionRedraw;

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

bitflags! {
    /// Action required after processing
    ///
    /// Some methods operate directly on a context ([`ConfigCx`] or [`EventCx`])
    /// while others don't reqiure a context but do require that some *action*
    /// is performed afterwards. This enum is used to convey that action.
    ///
    /// A `WindowAction` produced at run-time should be passed to a context, usually
    /// via [`EventState::action`] (to associate the `WindowAction` with a widget)
    /// or [`EventState::window_action`] (if no particular widget is relevant).
    ///
    /// A `WindowAction` produced before starting the GUI may be discarded, for
    /// example: `let _ = runner.config_mut().font.set_size(24.0);`.
    ///
    /// Two `WindowAction` values may be combined via bit-or (`a | b`).
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct WindowAction: u32 {
        /// The whole window requires redrawing
        ///
        /// See also [`EventState::redraw`].
        const REDRAW = 1 << 0;
        /// The current window should be closed
        ///
        /// See also [`EventState::exit`] which closes the UI (all windows).
        const CLOSE = 1 << 30;
    }
}

impl From<ActionRedraw> for WindowAction {
    #[inline]
    fn from(_: ActionRedraw) -> Self {
        WindowAction::REDRAW
    }
}
