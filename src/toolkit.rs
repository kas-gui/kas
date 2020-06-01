// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit interface
//!
//! In KAS, the "toolkit" is an external library handling system interfaces
//! (windowing and event translation) plus rendering. This allows KAS's core
//! to remain system-neutral.
//!
//! Note: although the choice of windowing library is left to the toolkit, for
//! convenience KAS is able to use several [winit] types.
//!
//! [winit]: https://github.com/rust-windowing/winit

use std::num::NonZeroU32;

use crate::string::{CowString, CowStringL};
use crate::{event, ThemeAction, ThemeApi};

/// Identifier for a window or pop-up
///
/// Identifiers should always be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WindowId(NonZeroU32);

impl WindowId {
    /// Construct a [`WindowId`]
    ///
    /// Only for toolkit use!
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    pub fn new(n: NonZeroU32) -> WindowId {
        WindowId(n)
    }
}

/// Action required after processing
///
/// This type is returned by many widgets on modification to self and is tracked
/// internally by [`event::Manager`] to determine which updates are needed to
/// the UI.
///
/// All variants are *progressive*: e.g. `Reconfigure` implies all actions
/// needed to handle `Popup`, `RegionMoved` and `Redraw`.
///
/// Two `TkAction` values may be combined by taking their maximum. Since this
/// is a common operation, the `+` operator is defined to do this job, together
/// with `+=` on `TkAction` and [`event::Manager`].
///
/// Users receiving a value of this type from a widget update method should
/// generally call `*mgr += action;` during event handling. Prior to
/// starting the event loop (`toolkit.run()`), these values can be ignored.
#[must_use]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum TkAction {
    /// No action needed
    None,
    /// Whole window requires redrawing
    ///
    /// Note that [`Manager::redraw`] can instead be used for more selective
    /// redrawing, if supported by the toolkit.
    ///
    /// [`Manager::redraw`]: crate::event::Manager::redraw
    Redraw,
    /// Some widgets within a region moved
    ///
    /// This action should be emitted when e.g. a scroll-region is moved or
    /// widget layout is adjusted to allow for the fact that coordinates
    /// (e.g. mouse position) have changed relative to widgets.
    // NOTE: one could specify a Rect here, but there's not much advantage
    RegionMoved,
    /// A pop-up opened/closed/needs resizing
    Popup,
    /// Whole window requires reconfiguring
    ///
    /// *Configuring* widgets assigns [`WidgetId`] identifiers and calls
    /// [`kas::WidgetConfig::configure`].
    ///
    /// [`WidgetId`]: crate::WidgetId
    /// [`event::Manager`]: crate::event::Manager
    Reconfigure,
    /// The window or pop-up should be closed
    Close,
    /// All windows should close (toolkit exit)
    CloseAll,
}

impl std::ops::Add for TkAction {
    type Output = Self;

    #[inline]
    fn add(self, rhs: TkAction) -> Self {
        self.max(rhs)
    }
}

impl std::ops::AddAssign for TkAction {
    #[inline]
    fn add_assign(&mut self, rhs: TkAction) {
        *self = (*self).max(rhs);
    }
}

/// Toolkit-specific window management and style interface.
///
/// This is implemented by a KAS toolkit on a window handle.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub trait TkWindow {
    /// Add a pop-up
    ///
    /// A pop-up may be presented as an overlay layer in the current window or
    /// via a new borderless window.
    ///
    /// Pop-ups support position hints: they are placed *next to* the specified
    /// `rect`, preferably in the given `direction`.
    fn add_popup(&mut self, popup: kas::Popup) -> WindowId;

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added via event
    /// processing, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId;

    /// Close a window
    fn close_window(&mut self, id: WindowId);

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`event::UpdateHandle`], across all
    /// windows, will receive an update.
    fn trigger_update(&mut self, handle: event::UpdateHandle, payload: u64);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<CowString>;

    /// Attempt to set clipboard contents
    fn set_clipboard<'c>(&mut self, content: CowStringL<'c>);

    /// Adjust the theme
    fn adjust_theme(&mut self, f: &mut dyn FnMut(&mut dyn ThemeApi) -> ThemeAction);

    /// Set the mouse cursor
    fn set_cursor_icon(&mut self, icon: event::CursorIcon);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn action_precedence() {
        assert!(TkAction::None < TkAction::Redraw);
        assert!(TkAction::Redraw < TkAction::Reconfigure);
        assert!(TkAction::Reconfigure < TkAction::Close);
        assert!(TkAction::Close < TkAction::CloseAll);
    }
}
