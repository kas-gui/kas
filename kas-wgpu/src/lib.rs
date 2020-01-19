// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

pub mod draw;
mod event_loop;
mod font;
mod shared;
mod theme;
mod window;

use std::{error, fmt};

use kas::WindowId;
use winit::error::OsError;
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::draw::DrawPipe;
use crate::shared::SharedState;
use window::Window;

pub use theme::SampleTheme;

pub use kas;
pub use wgpu_glyph as glyph;

/// Possible failures from constructing a [`Toolkit`]
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// No suitable graphics adapter found
    ///
    /// This can be a driver/configuration issue or hardware limitation. Note
    /// that for now, `wgpu` only supports DX11, DX12, Vulkan and Metal.
    NoAdapter,
    /// OS error during window creation
    Window(OsError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::NoAdapter => write!(f, "no suitable graphics adapter found"),
            Error::Window(e) => write!(f, "window creation error: {}", e),
        }
    }
}

impl error::Error for Error {}

impl From<OsError> for Error {
    fn from(ose: OsError) -> Self {
        Error::Window(ose)
    }
}

/// Builds a toolkit over a `winit::event_loop::EventLoop`.
pub struct Toolkit<T: kas::theme::Theme<DrawPipe>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<(WindowId, Window<T::Window>)>,
    shared: SharedState<T>,
}

impl<T: kas::theme::Theme<DrawPipe> + 'static> Toolkit<T> {
    /// Construct a new instance with default options.
    ///
    /// This chooses a low-power graphics adapter by preference.
    pub fn new(theme: T) -> Result<Self, Error> {
        Self::new_custom(theme, None)
    }

    /// Construct an instance with custom options
    ///
    /// The graphics adapter is chosen according to the given options. If `None`
    /// is supplied, a low-power adapter will be chosen.
    pub fn new_custom(
        theme: T,
        adapter_options: Option<&wgpu::RequestAdapterOptions>,
    ) -> Result<Self, Error> {
        Ok(Toolkit {
            el: EventLoop::with_user_event(),
            windows: vec![],
            shared: SharedState::new(theme, adapter_options)?,
        })
    }

    /// Assume ownership of and display a window
    ///
    /// This is a convenience wrapper around [`Toolkit::add_boxed`].
    ///
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    pub fn add<W: kas::Window + 'static>(&mut self, window: W) -> Result<WindowId, Error> {
        self.add_boxed(Box::new(window))
    }

    /// Add a boxed window directly
    pub fn add_boxed(&mut self, widget: Box<dyn kas::Window>) -> Result<WindowId, Error> {
        let window = winit::window::Window::new(&self.el)?;
        window.set_title(widget.title());
        let win = Window::new(&mut self.shared, window, widget);
        let id = self.shared.next_window_id();
        self.windows.push((id, win));
        Ok(id)
    }

    /// Create a proxy which can be used to update the UI from another thread
    pub fn create_proxy(&self) -> ToolkitProxy {
        ToolkitProxy {
            proxy: self.el.create_proxy(),
        }
    }

    /// Run the main loop.
    pub fn run(self) -> ! {
        let mut el = event_loop::Loop::new(self.windows, self.shared);
        self.el
            .run(move |event, elwt, control_flow| el.handle(event, elwt, control_flow))
    }
}

/// A proxy allowing control of a [`Toolkit`] from another thread.
///
/// Created by [`Toolkit::create_proxy`].
pub struct ToolkitProxy {
    proxy: EventLoopProxy<ProxyAction>,
}

/// Error type returned by [`ToolkitProxy`] functions.
///
/// This error occurs only if the [`Toolkit`] already terminated.
pub struct ClosedError;

impl ToolkitProxy {
    /// Close a specific window.
    pub fn close(&self, id: WindowId) -> Result<(), ClosedError> {
        self.proxy
            .send_event(ProxyAction::Close(id))
            .map_err(|_| ClosedError)
    }

    /// Close all windows and terminate the UI.
    pub fn close_all(&self) -> Result<(), ClosedError> {
        self.proxy
            .send_event(ProxyAction::CloseAll)
            .map_err(|_| ClosedError)
    }
}

#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
}
