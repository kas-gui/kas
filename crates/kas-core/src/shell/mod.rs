// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shell

mod common;
#[cfg(feature = "winit")] mod event_loop;
#[cfg(feature = "winit")] mod shared;
#[cfg(feature = "winit")] mod shell;
#[cfg(feature = "winit")] mod window;

#[cfg(feature = "winit")] use event_loop::Loop as EventLoop;
#[cfg(feature = "winit")] use shared::SharedState;
#[cfg(feature = "winit")] use window::Window;

pub(crate) use common::ShellWindow;
#[cfg(feature = "winit")] pub use common::WindowSurface;
pub use common::{Error, GraphicalShell, Result};
#[cfg(feature = "winit")]
pub use shell::{ClosedError, Proxy, Shell, ShellAssoc};
#[cfg(feature = "winit")]
pub extern crate raw_window_handle;

#[cfg(feature = "winit")]
enum PendingAction {
    AddPopup(winit::window::WindowId, kas::WindowId, kas::Popup),
    AddWindow(kas::WindowId, Box<dyn kas::Window>),
    CloseWindow(kas::WindowId),
    Update(kas::event::UpdateId, u64),
    Action(kas::Action),
}

#[cfg(feature = "winit")]
#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(kas::WindowId),
    Update(kas::event::UpdateId, u64),
}
