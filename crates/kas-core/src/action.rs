// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Action enum

#[allow(unused)]
use crate::event::{ConfigCx, EventCx, EventState};

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
    ///
    /// There is no `Action` value for accessibility updates. Instead, any of
    /// `REDRAW`, `REGION_MOVED`, `SCROLLED`, `SET_RECT`, `RESIZE` or
    /// `RECONFIGURE` applied to a target widget will cause that widget
    /// and descendants to update their accessibility sub-trees.
    /// Any of `SCROLLED`, `SET_RECT, `RESIZE` or `RECONFIGURE` applied to the
    /// root window (or without a target widget) will update the whole window's
    /// accessibility tree.
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct Action: u32 {
        /// The whole window requires redrawing
        ///
        /// See also [`EventState::redraw`].
        const REDRAW = 1 << 0;
        /// Some widgets within a region moved
        ///
        /// Used when a pop-up is closed or a region adjusted (e.g. scroll or switch
        /// tab) to update which widget is under the mouse cursor / touch events.
        /// Identifier is that of the parent widget/window encapsulating the region.
        ///
        /// Implies window redraw.
        ///
        /// See also [`EventState::region_moved`].
        const REGION_MOVED = 1 << 4;
        /// A widget was scrolled
        ///
        /// This is used for inter-widget communication (see `EditBox`). If not
        /// handled locally, it is handled identially to [`Self::SET_RECT`].
        const SCROLLED = 1 << 6;
        /// Reset size of all widgets without recalculating requirements
        const SET_RECT = 1 << 8;
        /// Resize all widgets in the window
        ///
        /// See also [`EventState::resize`].
        const RESIZE = 1 << 9;
        /// Update [`Dimensions`](crate::theme::dimensions::Dimensions) instances
        /// and theme configuration.
        ///
        /// Implies [`Action::RESIZE`].
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const THEME_UPDATE = 1 << 10;
        /// Reload per-window cache of event configuration
        ///
        /// Implies [`Action::UPDATE`].
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const EVENT_CONFIG = 1 << 11;
        /// Switch themes, replacing theme-window instances
        ///
        /// Implies [`Action::RESIZE`].
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
        const THEME_SWITCH = 1 << 12;
        /// Reconfigure all widgets of the window
        ///
        /// *Configuring* widgets assigns [`Id`](crate::Id) identifiers and calls
        /// [`Events::configure`](crate::Events::configure).
        ///
        /// Implies [`Action::UPDATE`] since widgets are updated on configure.
        const RECONFIGURE = 1 << 16;
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
