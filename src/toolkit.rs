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

use crate::{event, WidgetId};

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
    /// Whole window requires reconfiguring (implies redrawing)
    ///
    /// *Configuring* widgets assigns [`WidgetId`] identifiers, updates
    /// [`event::Manager`] state and resizes all widgets.
    Reconfigure,
    /// Window should be closed
    Close,
    /// All windows should close (toolkit exit)
    CloseAll,
}

/// Toolkit-specific window management and style interface.
///
/// This is implemented by a KAS toolkit on a window handle.
///
/// Users interact with this trait in a few cases, such as implementing widget
/// event handling. In these cases the user is *always* given an existing
/// reference to a `TkWindow`. Mostly this trait is only used internally.
pub trait TkWindow {
    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added via event
    /// processing, albeit without error handling.
    fn add_window(&mut self, widget: Box<dyn kas::Window>);

    /// Read access to the event manager state
    fn data(&self) -> &event::Manager;

    /// Update event manager data with a closure
    ///
    /// The closure should return true if this update may require a redraw.
    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::Manager) -> bool);

    /// Notify that a widget must be redrawn
    fn redraw(&mut self, id: WidgetId);

    /// Notify that a toolkit action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    fn send_action(&mut self, action: TkAction);

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    fn get_clipboard(&mut self) -> Option<String>;

    /// Attempt to set clipboard contents
    fn set_clipboard(&mut self, content: String);
}
