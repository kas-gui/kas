// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shell

use thiserror::Error;
use winit::error::OsError;

mod event_loop;
mod shared;
mod shell;
mod window;

use crate::event::UpdateId;
use crate::{Action, WindowId};
use event_loop::Loop as EventLoop;
use shared::SharedState;
use window::Window;

pub use shell::{ClosedError, GraphicalShell, Proxy, Shell, ShellAssoc};
pub(crate) use window::ShellWindow;
pub use window::WindowSurface;
pub extern crate raw_window_handle;

/// Possible failures from constructing a [`Shell`]
///
/// Some variants are undocumented. Users should not match these variants since
/// they are not considered part of the public API.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    /// Failure from the graphics sub-system
    #[error("error from graphics sub-system")]
    Graphics(Box<dyn std::error::Error + 'static>),

    /// Config load/save error
    #[error("config load/save error")]
    Config(#[from] kas::config::Error),
    #[doc(hidden)]

    /// OS error during window creation
    #[error("operating system error")]
    Window(#[from] OsError),
}

/// A `Result` type representing `T` or [`enum@Error`]
pub type Result<T> = std::result::Result<T, Error>;

enum PendingAction {
    AddPopup(winit::window::WindowId, WindowId, kas::Popup),
    AddWindow(WindowId, Box<dyn kas::Window>),
    CloseWindow(WindowId),
    Update(kas::event::UpdateId, u64),
    Action(Action),
}

#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Update(UpdateId, u64),
}
