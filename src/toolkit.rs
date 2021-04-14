// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface
//!
//! This module provides the primary interface between the KAS toolkit and a
//! KAS shell, though it is not the only interface. A KAS shell connects to the
//! operating system (or further abstraction layers) by implementing
//! [`ShellWindow`], the family of draw traits in [`kas::draw`], and
//! constructing and using an event manager ([`kas::event::ManagerState`]).
//! The shell also provides the entrypoint, a type named `Toolkit`.

use std::num::NonZeroU32;
use std::rc::Rc;

use crate::draw::{SizeHandle, ThemeAction, ThemeApi};
use crate::event;
use crate::event::UpdateHandle;
use crate::updatable::Updatable;

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
    pub fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

bitflags! {
    /// Action required after processing
    ///
    /// This type is returned by many widgets on modification to self and is tracked
    /// internally by [`event::Manager`] to determine which updates are needed to
    /// the UI.
    ///
    /// Two `TkAction` values may be combined via bit-or (`a | b`). Bit-or
    /// assignments are supported by both `TkAction` and [`event::Manager`].
    ///
    /// Users receiving a value of this type from a widget update method should
    /// generally call `*mgr |= action;` during event handling. Prior to
    /// starting the event loop (`toolkit.run()`), these values can be ignored.
    #[must_use]
    #[derive(Default)]
    pub struct TkAction: u32 {
        /// The whole window requires redrawing
        ///
        /// Note that [`event::Manager::redraw`] can instead be used for more
        /// selective redrawing.
        const REDRAW = 1 << 0;
        /// Some widgets within a region moved
        ///
        /// Used when a pop-up is closed or a region adjusted (e.g. scroll or switch
        /// tab) to update which widget is under the mouse cursor / touch events.
        /// Identifier is that of the parent widget/window encapsulating the region.
        ///
        /// Implies window redraw.
        const REGION_MOVED = 1 << 1;
        /*
        /// A pop-up opened/closed/needs resizing
        Popup,
        */
        /// Reset size of all widgets without recalculating requirements
        const SET_SIZE = 1 << 8;
        /// Resize all widgets
        const RESIZE = 1 << 9;
        /// Window requires reconfiguring
        ///
        /// *Configuring* widgets assigns [`WidgetId`] identifiers and calls
        /// [`kas::WidgetConfig::configure`].
        ///
        /// [`WidgetId`]: crate::WidgetId
        const RECONFIGURE = 1 << 16;
        /// The current window or pop-up should be closed
        const CLOSE = 1 << 30;
        /// Close all windows and exit
        const EXIT = 1 << 31;
    }
}

/// Shell-specific window management and style interface.
///
/// This is implemented by a KAS shell, per window.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
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
    fn add_popup(&mut self, popup: kas::Popup) -> Option<WindowId>;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added from an
    /// event handler, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Register `data` to be updated when an update with the given `handle` is triggered
    fn update_shared_data(&mut self, handle: UpdateHandle, data: Rc<dyn Updatable>);

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateHandle`], across all
    /// windows, will receive an update.
    fn trigger_update(&mut self, handle: UpdateHandle, payload: u64);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard<'c>(&mut self, content: String);

    /// Adjust the theme
    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeApi) -> ThemeAction);

    /// Access a [`SizeHandle`]
    ///
    /// Implementations should call the given function argument once; not doing
    /// so is memory-safe but will cause a panic when `size_handle` is called.
    /// User-code *must not* depend on `f` being called for memory safety.
    fn size_handle(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle));

    /// Set the mouse cursor
    fn set_cursor_icon(&mut self, icon: event::CursorIcon);
}
