// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window handle interface

use std::borrow::Cow;

use crate::runner::RunnerT;
use crate::{event::EventState, window::Decorations};
use raw_window_handle::{self as rwh, HandleError, HasDisplayHandle, HasWindowHandle};

/// A handle to a top-level window
pub struct TopWindow<'a> {
    pub(super) runner: &'a mut dyn RunnerT,
    pub(super) window: &'a dyn winit::window::Window,
    pub(super) state: &'a mut EventState,
}

impl<'a> TopWindow<'a> {
    /// Set the window title
    pub fn set_title<'s>(&'s mut self, title: impl Into<Cow<'s, str>>) {
        let title = title.into();
        if self.state.decorations == Decorations::Server {
            self.window.set_title(title.as_ref());
        } else if self.state.decorations == Decorations::Toolkit {
            self.state.window_title = title.into_owned();
            self.runner.update(self.state.window_id);
        }
    }

    /// Test whether the window is minimized
    ///
    /// Returns `None` when it was not possible to determine this.
    #[inline]
    pub fn is_minimized(&mut self) -> Option<bool> {
        self.window.is_minimized()
    }

    /// Minimize the window
    ///
    /// Setting `minimized = true` closes all popups and minimizes the top-level
    /// window.
    pub fn set_minimized(&mut self, minimized: bool) {
        if minimized {
            self.state.close_all_popups(self.runner);
        }
        self.window.set_minimized(minimized);
    }

    /// Test whether the window is maximized
    #[inline]
    pub fn is_maximized(&mut self) -> bool {
        self.window.is_maximized()
    }

    /// Set the maximized status
    #[inline]
    pub fn set_maximized(&mut self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    /// Request that the window be closed
    #[inline]
    pub fn close(&mut self) {
        self.state.action_close = Some(crate::ActionClose);
    }
}

impl<'a> HasWindowHandle for TopWindow<'a> {
    #[inline]
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, HandleError> {
        self.window.window_handle()
    }
}

impl<'a> HasDisplayHandle for TopWindow<'a> {
    #[inline]
    fn display_handle(&self) -> Result<rwh::DisplayHandle<'_>, HandleError> {
        self.window.display_handle()
    }
}
