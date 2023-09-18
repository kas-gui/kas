// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shell

mod common;
#[cfg(winit)] mod event_loop;
#[cfg(winit)] mod shared;
#[cfg(winit)] mod shell;
#[cfg(winit)] mod window;

#[cfg(winit)] use crate::WindowId;
#[cfg(winit)] use event_loop::Loop as EventLoop;
#[cfg(winit)]
pub(crate) use shared::{SharedState, ShellSharedErased};
#[cfg(winit)] use shell::PlatformWrapper;
#[cfg(winit)]
pub(crate) use window::{Window, WindowDataErased};

pub use common::{Error, Platform, Result};
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub use common::{GraphicalShell, WindowSurface};
#[cfg(winit)]
pub use shell::{ClosedError, Proxy, Shell, ShellAssoc};
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
pub extern crate raw_window_handle;

// TODO(opt): Clippy is probably right that we shouldn't copy a large value
// around (also applies when constructing a shell::Window).
#[allow(clippy::large_enum_variant)]
#[crate::autoimpl(Debug)]
#[cfg(winit)]
enum Pending<A: kas::AppData, S: WindowSurface, T: kas::theme::Theme<S::Shared>> {
    AddPopup(WindowId, WindowId, kas::PopupDescriptor),
    // NOTE: we don't need S, T here if we construct the Window later.
    // But this way we can pass a single boxed value.
    AddWindow(WindowId, Box<Window<A, S, T>>),
    CloseWindow(WindowId),
    Action(kas::Action),
}

#[cfg(winit)]
#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Message(kas::erased::SendErased),
    WakeAsync,
}
