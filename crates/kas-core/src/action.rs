// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Action enum

#[allow(unused)]
use crate::event::{ConfigCx, EventCx, EventState};
use std::ops::{BitOr, BitOrAssign, Deref};

/// Action: widget has moved/opened/closed
///
/// When the state is `true`, this indicates that the following should happen:
///
/// -   Re-probe which widget is under the mouse / any touch instance / any
///     other location picker since widgets may have moved
/// -   Redraw the window
#[must_use]
#[derive(Copy, Clone, Debug, Default)]
pub struct ActionMoved(pub bool);

impl BitOr for ActionMoved {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        ActionMoved(self.0 | rhs.0)
    }
}

impl BitOrAssign for ActionMoved {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Deref for ActionMoved {
    type Target = bool;
    #[inline]
    fn deref(&self) -> &bool {
        &self.0
    }
}

/// Action: widget must be resized
#[must_use]
#[derive(Copy, Clone, Debug, Default)]
pub struct ActionResize(pub bool);

impl BitOr for ActionResize {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        ActionResize(self.0 | rhs.0)
    }
}

impl BitOrAssign for ActionResize {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Deref for ActionResize {
    type Target = bool;
    #[inline]
    fn deref(&self) -> &bool {
        &self.0
    }
}

bitflags! {
    /// Action required after processing
    ///
    /// Some methods operate directly on a context ([`ConfigCx`] or [`EventCx`])
    /// while others don't reqiure a context but do require that some *action*
    /// is performed afterwards. This enum is used to convey that action.
    ///
    /// An `Action` produced at run-time should be passed to a context, usually
    /// via [`EventState::action`] (to associate the `Action` with a widget)
    /// or [`EventState::window_action`] (if no particular widget is relevant).
    ///
    /// An `Action` produced before starting the GUI may be discarded, for
    /// example: `let _ = runner.config_mut().font.set_size(24.0);`.
    ///
    /// Two `Action` values may be combined via bit-or (`a | b`).
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct Action: u32 {
        /// The whole window requires redrawing
        ///
        /// See also [`EventState::redraw`].
        const REDRAW = 1 << 0;
        /// Resize all widgets in the window
        ///
        /// Solves for size rules, applies, and updates bounds on window size.
        ///
        /// See also [`EventState::resize`].
        const RESIZE = 1 << 9;
        /// Update per-window theme data from theme configuration.
        ///
        /// Causes a resize if required by the configuration changes.
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const THEME_UPDATE = 1 << 10;
        /// Reload per-window cache of configuration data
        ///
        /// Implies reconfiguration of all widgets.
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const CONFIG_UPDATE = 1 << 11;
        /// Switch themes, replacing theme-window instances
        ///
        /// Implies [`Action::RESIZE`].
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const THEME_SWITCH = 1 << 12;
        /// Update all widgets
        ///
        /// This is a notification that input data has changed.
        const UPDATE = 1 << 17;
        /// The current window should be closed
        ///
        /// See also [`EventState::exit`] which closes the UI (all windows).
        const CLOSE = 1 << 30;
    }
}
