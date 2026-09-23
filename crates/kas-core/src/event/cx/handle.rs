// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window handle interface

use raw_window_handle::{self as rwh, HandleError, HasDisplayHandle, HasWindowHandle};

/// A handle to a top-level window
pub struct TopWindow<'a> {
    pub(super) window: &'a dyn winit::window::Window,
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
