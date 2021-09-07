// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! KAS shell over [winit] and [WebGPU]
//!
//! This crate implements a KAS shell (backend) using [WebGPU] for
//! GPU-accelerated rendering and [winit] for windowing, thus it should be
//! portable to most desktop and potentially also mobile platforms.
//!
//! This crate supports themes via the [`kas_theme`] crate, including shaded
//! drawing.
//!
//! Custom GPU-accelerated drawing is supported via [`draw::CustomPipe`]
//! (see the [Mandlebrot example](https://github.com/kas-gui/kas/blob/master/kas-wgpu/examples/mandlebrot.rs)).
//!
//! By default, some environment variables are read for configuration.
//! See [`options::Options::from_env`] for documentation.
//!
//! [WebGPU]: https://github.com/gfx-rs/wgpu-rs
//! [winit]: https://github.com/rust-windowing/winit
//! [clipboard]: https://crates.io/crates/clipboard

#![cfg_attr(feature = "gat", feature(generic_associated_types))]

pub mod draw;
mod event_loop;
pub mod options;
mod shared;
mod window;

use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;

use kas::event::UpdateHandle;
use kas::WindowId;
use kas_theme::Theme;
use winit::error::OsError;
use winit::event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};

use crate::draw::{CustomPipe, CustomPipeBuilder, DrawPipe};
use crate::shared::SharedState;
use window::Window;

pub use options::Options;

pub use kas;
pub use kas_theme as theme;
pub use wgpu;

/// Possible failures from constructing a [`Toolkit`]
///
/// Some variants are undocumented. Users should not match these variants since
/// they are not considered part of the public API.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    /// No suitable graphics adapter found
    ///
    /// This can be a driver/configuration issue or hardware limitation. Note
    /// that for now, `wgpu` only supports DX11, DX12, Vulkan and Metal.
    #[error("no graphics adapter found")]
    NoAdapter,
    /// Config load/save error
    #[error("config load/save error")]
    Config(#[from] kas::config::Error),
    #[doc(hidden)]
    /// OS error during window creation
    #[error("operating system error")]
    Window(#[from] OsError),
}

impl From<wgpu::RequestDeviceError> for Error {
    fn from(_: wgpu::RequestDeviceError) -> Self {
        Error::NoAdapter
    }
}

fn warn_about_error(msg: &str, mut error: &dyn std::error::Error) {
    log::warn!("{}: {}", msg, error);
    while let Some(source) = error.source() {
        log::warn!("Source: {}", source);
        error = source;
    }
}

/// A toolkit over winit and WebGPU
///
/// Constructing the toolkit with [`Toolkit::new`] or [`Toolkit::new_custom`]
/// reads configuration (depending on passed options or environment variables)
/// and initialises the font database. Note that this database is a global
/// singleton and some widgets and other library code may expect fonts to have
/// been initialised first.
///
/// All KAS shells are expected to provide a similar `Toolkit` type and API.
/// There is no trait abstraction over this API simply because there is very
/// little reason to do so (and some reason not to: KISS).
pub struct Toolkit<C: CustomPipe, T: Theme<DrawPipe<C>>> {
    el: EventLoop<ProxyAction>,
    windows: Vec<Window<C, T>>,
    shared: SharedState<C, T>,
}

impl<T: Theme<DrawPipe<()>> + 'static> Toolkit<(), T>
where
    T::Window: kas_theme::Window,
{
    /// Construct a new instance with default options.
    ///
    /// Environment variables may affect option selection; see documentation
    /// of [`Options::from_env`]. KAS config is provided by
    /// [`Options::read_config`].
    #[inline]
    pub fn new(theme: T) -> Result<Self, Error> {
        Self::new_custom((), theme, Options::from_env())
    }
}

impl<C: CustomPipe, T: Theme<DrawPipe<C>> + 'static> Toolkit<C, T>
where
    T::Window: kas_theme::Window,
{
    /// Construct an instance with custom options
    ///
    /// The `custom` parameter accepts a custom draw pipe (see [`CustomPipeBuilder`]).
    /// Pass `()` if you don't have one.
    ///
    /// The [`Options`] parameter allows direct specification of shell options;
    /// usually, these are provided by [`Options::from_env`].
    ///
    /// KAS config is provided by [`Options::read_config`] and `theme` is
    /// configured through [`Options::init_theme_config`].
    #[inline]
    pub fn new_custom<CB: CustomPipeBuilder<Pipe = C>>(
        custom: CB,
        mut theme: T,
        options: Options,
    ) -> Result<Self, Error> {
        let el = EventLoop::with_user_event();

        options.init_theme_config(&mut theme)?;
        let config = match options.read_config() {
            Ok(config) => config,
            Err(error) => {
                warn_about_error("Failed to save read", &error);
                Default::default()
            }
        };
        let config = Rc::new(RefCell::new(config));
        let scale_factor = find_scale_factor(&el);
        Ok(Toolkit {
            el,
            windows: vec![],
            shared: SharedState::new(custom, theme, options, config, scale_factor)?,
        })
    }

    /// Construct an instance with custom options and config
    ///
    /// This is like [`Toolkit::new_custom`], but allows KAS config to be
    /// specified directly, instead of loading via [`Options::read_config`].
    ///
    /// Unlike other the constructors, this method does not configure the theme.
    /// The user should call [`Options::init_theme_config`] before this method.
    #[inline]
    pub fn new_custom_config<CB: CustomPipeBuilder<Pipe = C>>(
        custom: CB,
        theme: T,
        options: Options,
        config: Rc<RefCell<kas::event::Config>>,
    ) -> Result<Self, Error> {
        let el = EventLoop::with_user_event();
        let scale_factor = find_scale_factor(&el);
        Ok(Toolkit {
            el,
            windows: vec![],
            shared: SharedState::new(custom, theme, options, config, scale_factor)?,
        })
    }

    /// Access the theme by ref
    #[inline]
    pub fn theme(&self) -> &T {
        &self.shared.theme
    }

    /// Access the theme by ref mut
    #[inline]
    pub fn theme_mut(&mut self) -> &mut T {
        &mut self.shared.theme
    }

    /// Assume ownership of and display a window
    ///
    /// This is a convenience wrapper around [`Toolkit::add_boxed`].
    ///
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    #[inline]
    pub fn add<W: kas::Window + 'static>(&mut self, window: W) -> Result<WindowId, Error> {
        self.add_boxed(Box::new(window))
    }

    /// Assume ownership of and display a window, inline
    ///
    /// This is a convenience wrapper around [`Toolkit::add_boxed`].
    ///
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    #[inline]
    pub fn with<W: kas::Window + 'static>(mut self, window: W) -> Result<Self, Error> {
        self.add_boxed(Box::new(window))?;
        Ok(self)
    }

    /// Add a boxed window directly
    pub fn add_boxed(&mut self, widget: Box<dyn kas::Window>) -> Result<WindowId, Error> {
        let id = self.shared.next_window_id();
        let win = Window::new(&mut self.shared, &self.el, id, widget)?;
        self.windows.push(win);
        Ok(id)
    }

    /// Add a boxed window directly, inline
    #[inline]
    pub fn with_boxed(mut self, widget: Box<dyn kas::Window>) -> Result<Self, Error> {
        self.add_boxed(widget)?;
        Ok(self)
    }

    /// Create a proxy which can be used to update the UI from another thread
    pub fn create_proxy(&self) -> ToolkitProxy {
        ToolkitProxy {
            proxy: self.el.create_proxy(),
        }
    }

    /// Run the main loop.
    #[inline]
    pub fn run(self) -> ! {
        let mut el = event_loop::Loop::new(self.windows, self.shared);
        self.el
            .run(move |event, elwt, control_flow| el.handle(event, elwt, control_flow))
    }
}

fn find_scale_factor<T>(el: &EventLoopWindowTarget<T>) -> f64 {
    if let Some(mon) = el.primary_monitor() {
        return mon.scale_factor();
    }
    if let Some(mon) = el.available_monitors().next() {
        return mon.scale_factor();
    }
    1.0
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
