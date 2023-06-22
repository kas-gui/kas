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
#[cfg(feature = "winit")] use shell::PlatformWrapper;
#[cfg(feature = "winit")] use window::Window;

pub(crate) use common::ShellWindow;
pub use common::{Error, GraphicalShell, Platform, Result, WindowSurface};
#[cfg(feature = "winit")]
pub use shell::{ClosedError, Proxy, Shell, ShellAssoc};
pub extern crate raw_window_handle;

// TODO(opt): Clippy is probably right that we shouldn't copy a large value
// around (also applies when constructing a shell::Window).
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "winit")]
enum PendingAction {
    AddPopup(winit::window::WindowId, kas::WindowId, kas::Popup),
    AddWindow(kas::WindowId, kas::Window),
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
    WakeAsync,
}
