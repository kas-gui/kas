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

use crate::draw::DrawShared;
use crate::event;
use crate::event::UpdateId;
use crate::theme::{ThemeControl, ThemeSize};
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
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn new(n: NonZeroU32) -> WindowId {
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
    /// Two `TkAction` values may be combined via bit-or (`a | b`). Bit-or
    /// assignments are supported by both `TkAction` and [`event::EventMgr`].
    ///
    /// Users receiving a value of this type from a widget update method should
    /// usually handle with `*mgr |= action;`. Before the event loop starts
    /// (`toolkit.run()`) or if the widget in question is not part of a UI these
    /// values can be ignored.
    #[must_use]
    #[derive(Default)]
    pub struct TkAction: u32 {
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

/// Shell-specific window management and style interface.
///
/// This is implemented by a KAS shell, per window.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
pub trait ShellWindow {
    /// Add a pop-up
    ///
    /// A pop-up may be presented as an overlay layer in the current window or
    /// via a new borderless window.
    ///
    /// Pop-ups support position hints: they are placed *next to* the specified
    /// `rect`, preferably in the given `direction`.
    ///
    /// Returns `None` if window creation is not currently available (but note
    /// that `Some` result does not guarantee the operation succeeded).
    fn add_popup(&mut self, popup: crate::Popup) -> Option<WindowId>;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added from an
    /// event handler, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn crate::Window>) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateId`], across all
    /// windows, will receive an update.
    fn update_all(&mut self, id: UpdateId, payload: u64);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard(&mut self, content: String);

    /// Adjust the theme
    ///
    /// Note: theme adjustments apply to all windows, as does the [`TkAction`]
    /// returned from the closure.
    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeControl) -> TkAction);

    /// Access [`ThemeSize`] and [`DrawShared`] objects
    ///
    /// Implementations should call the given function argument once; not doing
    /// so is memory-safe but will cause panics in `EventMgr` methods.
    /// User-code *must not* depend on `f` being called for memory safety.
    fn size_and_draw_shared(&mut self, f: &mut dyn FnMut(&mut dyn ThemeSize, &mut dyn DrawShared));

    /// Set the mouse cursor
    fn set_cursor_icon(&mut self, icon: event::CursorIcon);
}
