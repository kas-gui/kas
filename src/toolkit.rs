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

use crate::geom::{AxisInfo, SizeRules};
use crate::{Widget, WidgetId};

/// Toolkit actions needed after event handling, if any.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum TkAction {
    /// No action needed
    None,
    /// Whole window requires redrawing
    ///
    /// Note that [`TkWindow::redraw`] can instead be used for more selective
    /// redrawing, if supported by the toolkit.
    Redraw,
    /// Whole window requires reconfigure *and* redrawing
    Reconfigure,
    /// Window should be closed
    Close,
    /// All windows should close (toolkit exit)
    CloseAll,
}

/// Toolkit-specific window management and style interface.
///
/// This is implemented by a KAS toolkit on a window handle. Since each window
/// is assumed to have uniform styling and this styling is provided by the
/// toolkit, this interface includes widget styling ([`size_rules`]).
///
/// Users interact with this trait in a few cases, such as implementing widget
/// event handling. In these cases the user is *always* given an existing
/// reference to a `TkWindow`. Mostly this trait is only used internally.
pub trait TkWindow {
    /// Get the widget's size preferences
    ///
    /// See documentation of [`crate::Layout::size_rules`].
    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules;

    /// Notify that a widget must be redrawn
    fn redraw(&mut self, widget: &dyn Widget);

    /// Notify that a toolkit action should happen
    ///
    /// Allows signalling application exit, etc.
    fn send_action(&mut self, action: TkAction);

    /// Get the widget under the mouse
    fn hover(&self) -> Option<WidgetId>;
    /// Set the widget under the mouse
    fn set_hover(&mut self, id: Option<WidgetId>);

    /// Get the widget under the mouse when a left-click starts
    fn click_start(&self) -> Option<WidgetId>;
    /// Set the widget under the mouse when a left-click starts
    fn set_click_start(&mut self, id: Option<WidgetId>);
}
