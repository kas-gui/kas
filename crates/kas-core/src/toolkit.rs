// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface
//!
//! This module provides the primary interface between the KAS toolkit and a
//! KAS shell, though it is not the only interface. A KAS shell connects to the
//! operating system (or further abstraction layers) by implementing
//! [`ShellWindow`], the family of draw traits in [`crate::draw`], and
//! constructing and using an event manager ([`crate::event::EventState`]).
//! The shell also provides the entrypoint, a type named `Toolkit`.

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

bitflags! {
    /// Action required after processing
    ///
    /// This type is returned by many widgets on modification to self and is tracked
    /// internally by [`event::EventMgr`] to determine which updates are needed to
    /// the UI.
    ///
    /// Two `Action` values may be combined via bit-or (`a | b`). Bit-or
    /// assignments are supported by both `Action` and [`event::EventMgr`].
    ///
    /// Users receiving a value of this type from a widget update method should
    /// usually handle with `*mgr |= action;`. Before the event loop starts
    /// (`toolkit.run()`) or if the widget in question is not part of a UI these
    /// values can be ignored.
    #[must_use]
    #[derive(Default)]
    pub struct Action: u32 {
        /// No flags
        ///
        /// This is a [zero flag](https://docs.rs/bitflags/latest/bitflags/#zero-flags).
        const EMPTY = 0;
        /// The whole window requires redrawing
        ///
        /// Note that [`event::EventMgr::redraw`] can instead be used for more
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
        /*
        /// A pop-up opened/closed/needs resizing
        Popup,
        */
        /// Reset size of all widgets without recalculating requirements
        const SET_SIZE = 1 << 8;
        /// Resize all widgets in the window
        const RESIZE = 1 << 9;
        /// Update theme memory
        const THEME_UPDATE = 1 << 10;
        /// Reconfigure all widgets of the window
        ///
        /// *Configuring* widgets assigns [`WidgetId`] identifiers and calls
        /// [`crate::Widget::configure`].
        ///
        /// [`WidgetId`]: crate::WidgetId
        const RECONFIGURE = 1 << 16;
        /// The current window should be closed
        const CLOSE = 1 << 30;
        /// Close all windows and exit
        const EXIT = 1 << 31;
    }
}
