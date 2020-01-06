// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

pub mod draw;
mod event;
mod font;
mod theme;
mod window;

use log::info;
use std::{error, fmt};

use winit::error::OsError;
use winit::event_loop::EventLoop;

use crate::draw::DrawPipe;
use window::Window;

pub use theme::SampleTheme;

pub use kas;
pub use wgpu_glyph as glyph;

/// State shared between windows
struct SharedState<T> {
    device: wgpu::Device,
    queue: wgpu::Queue,
    theme: T,
}

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
pub struct Toolkit<T: kas::theme::Theme<DrawPipe>, U: 'static> {
    el: EventLoop<U>,
    windows: Vec<Window<T::Window>>,
    shared: SharedState<T>,
}

impl<T: kas::theme::Theme<DrawPipe> + 'static> Toolkit<T, ()> {
    /// Construct a new instance with default options.
    ///
    /// This chooses a low-power graphics adapter by preference.
    pub fn new(theme: T) -> Result<Self, Error> {
        Toolkit::<T, ()>::new_custom(theme, None)
    }
}

impl<T: kas::theme::Theme<DrawPipe> + 'static, U: 'static> Toolkit<T, U> {
    /// Construct an instance with custom options
    ///
    /// The graphics adapter is chosen according to the given options. If `None`
    /// is supplied, a low-power adapter will be chosen.
    ///
    /// The event loop supports user events of type `T`. Refer to winit's
    /// documentation of `EventLoop::with_user_event` for details.
    /// If not using user events, it may be necessary to force this type:
    /// ```
    /// let theme = kas_wgpu::SampleTheme::new();
    /// let toolkit = kas_wgpu::Toolkit::<_, ()>::new_custom(theme, None);
    /// ```
    pub fn new_custom(
        theme: T,
        adapter_options: Option<&wgpu::RequestAdapterOptions>,
    ) -> Result<Self, Error> {
        let adapter_options = adapter_options.unwrap_or(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::PRIMARY,
        });
        let adapter = match wgpu::Adapter::request(adapter_options) {
            Some(a) => a,
            None => return Err(Error::NoAdapter),
        };
        info!("Using graphics adapter: {}", adapter.get_info().name);

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        Ok(Toolkit {
            el: EventLoop::with_user_event(),
            windows: vec![],
            shared: SharedState {
                device,
                queue,
                theme,
            },
        })
    }

    /// Assume ownership of and display a window.
    ///
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    pub fn add<W: kas::Window + 'static>(&mut self, window: W) -> Result<(), Error> {
        self.add_boxed(Box::new(window))
    }

    /// Add a boxed window directly
    pub fn add_boxed(&mut self, widget: Box<dyn kas::Window>) -> Result<(), Error> {
        let window = winit::window::Window::new(&self.el)?;
        window.set_title(widget.title());
        let win = Window::new(&mut self.shared, window, widget);
        self.windows.push(win);
        Ok(())
    }

    /// Run the main loop.
    pub fn run(self) -> ! {
        let mut el = event::Loop::new(self.windows, self.shared);
        self.el
            .run(move |event, elwt, control_flow| el.handle(event, elwt, control_flow))
    }
}
