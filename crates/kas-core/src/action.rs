// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Action enum

bitflags! {
    /// Action required after processing
    ///
    /// Some methods operate directly on a context ([`ConfigCx`] or [`EventCx`])
    /// while others don't reqiure a context but do require that some *action*
    /// is performed afterwards. This enum is used to convey that action.
    ///
    /// An `Action` should be passed to a context: `cx.action(self.id(), action)`
    /// (assuming `self` is a widget).
    ///
    /// Two `Action` values may be combined via bit-or (`a | b`).
    #[must_use]
    #[derive(Copy, Clone, Debug, Default)]
    pub struct Action: u32 {
        /// The whole window requires redrawing
        ///
        /// Note that [`event::EventCx::redraw`] can instead be used for more
        /// selective redrawing.
        const REDRAW = 1 << 0;
        /// Some widgets within a region moved
        ///
        /// Used when a pop-up is closed or a region adjusted (e.g. scroll or switch
        /// tab) to update which widget is under the mouse cursor / touch events.
        /// Identifier is that of the parent widget/window encapsulating the region.
        ///
        /// Implies window redraw.
        const REGION_MOVED = 1 << 4;
        /// A widget was scrolled
        ///
        /// This is used for inter-widget communication (see `EditBox`). If not
        /// handled locally, it is handled identially to [`Self::SET_RECT`].
        const SCROLLED = 1 << 6;
        /// Reset size of all widgets without recalculating requirements
        const SET_RECT = 1 << 8;
        /// Resize all widgets in the window
        const RESIZE = 1 << 9;
        /// Update theme memory
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
        const THEME_UPDATE = 1 << 10;
        /// Reload per-window cache of event configuration
        #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
        #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
        const EVENT_CONFIG = 1 << 11;
        /// Reconfigure all widgets of the window
        ///
        /// *Configuring* widgets assigns [`Id`](crate::Id) identifiers and calls
        /// [`Events::configure`](crate::Events::configure).
        const RECONFIGURE = 1 << 16;
        /// Update all widgets
        ///
        /// This is a notification that input data has changed.
        const UPDATE = 1 << 17;
        /// The current window should be closed
        const CLOSE = 1 << 30;
        /// Close all windows and exit
        const EXIT = 1 << 31;
    }
}
