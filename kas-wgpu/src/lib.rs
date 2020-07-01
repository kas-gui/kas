// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for KAS targeting winit + WebGPU
//!
//! This crate provides an implementation of KAS, using
//! [WebGPU](https://github.com/gfx-rs/wgpu-rs) for GPU-based rendering.
//!
//! Windowing is provided by [winit](https://github.com/rust-windowing/winit/).
//! Clipboard functionality is (currently) provided by
//! [clipboard](https://crates.io/crates/clipboard).

#![cfg_attr(feature = "gat", feature(generic_associated_types))]

pub mod draw;
mod event_loop;
pub mod options;
mod shared;
mod window;

use std::{error, fmt};

use kas::event::UpdateHandle;
use kas::WindowId;
use kas_theme::Theme;
use winit::error::OsError;
use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::draw::{CustomPipe, CustomPipeBuilder, DrawPipe};
use crate::shared::SharedState;
use window::Window;

pub use options::Options;

pub use kas;
pub use kas_theme as theme;
pub use wgpu;
pub use wgpu_glyph as glyph;

/// Possible failures from constructing a [`Toolkit`]
///
/// Some variants are undocumented. Users should not match these variants since
/// they are not considered part of the public API.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// No suitable graphics adapter found
    ///
    /// This can be a driver/configuration issue or hardware limitation. Note
    /// that for now, `wgpu` only supports DX11, DX12, Vulkan and Metal.
    NoAdapter,
    #[doc(hidden)]
    /// Shaders failed to compile (likely internal issue)
    ShaderCompilation(shaderc::Error),
    /// OS error during window creation
    Window(OsError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::NoAdapter => write!(f, "no suitable graphics adapter found"),
            Error::ShaderCompilation(e) => write!(f, "shader compilation failed: {}", e),
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

impl From<shaderc::Error> for Error {
    fn from(e: shaderc::Error) -> Self {
        Error::ShaderCompilation(e)
    }
}

/// Builds a toolkit over a `winit::event_loop::EventLoop`.
pub struct Toolkit<C: CustomPipe, T: Theme<DrawPipe<C>>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<Window<C::Window, T::Window>>,
    shared: SharedState<C, T>,
}

impl<T: Theme<DrawPipe<()>> + 'static> Toolkit<(), T>
where
    T::Window: kas_theme::Window,
{
    /// Construct a new instance with default options.
    ///
    /// Environment variables may affect option selection; see documentation
    /// of [`Options::from_env`].
    pub fn new(theme: T) -> Result<Self, Error> {
        Self::new_custom((), theme, Options::from_env())
    }
}

impl<C: CustomPipe + 'static, T: Theme<DrawPipe<C>> + 'static> Toolkit<C, T>
where
    T::Window: kas_theme::Window,
{
    /// Construct an instance with custom options
    ///
    /// The `custom` parameter accepts a custom draw pipe (see [`CustomPipeBuilder`]).
    /// Pass `()` if you don't have one.
    ///
    /// The [`Options`] parameter allows direct specification of toolkit
    /// options; usually, these are provided by [`Options::from_env`].
    pub fn new_custom<CB: CustomPipeBuilder<Pipe = C>>(
        custom: CB,
        theme: T,
        options: Options,
    ) -> Result<Self, Error> {
        let el = EventLoop::with_user_event();
        let scale_factor = el.primary_monitor().scale_factor();
        Ok(Toolkit {
            el,
            windows: vec![],
            shared: SharedState::new(custom, theme, options, scale_factor)?,
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
        let id = self.shared.next_window_id();
        let win = Window::new(&mut self.shared, &self.el, id, widget)?;
        self.windows.push(win);
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

    /// Trigger an update handle
    pub fn trigger_update(&self, handle: UpdateHandle, payload: u64) -> Result<(), ClosedError> {
        self.proxy
            .send_event(ProxyAction::Update(handle, payload))
            .map_err(|_| ClosedError)
    }
}

#[derive(Debug)]
enum ProxyAction {
    CloseAll,
    Close(WindowId),
    Update(UpdateHandle, u64),
}
