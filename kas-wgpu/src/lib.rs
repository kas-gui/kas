// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

mod colour;
mod event;
mod font;
mod render;
mod tri_pipe;
mod vertex;
mod window;

pub use window::Window;

use winit::error::OsError;
use winit::event_loop::EventLoop;

/// Builds a toolkit over a `winit::event_loop::EventLoop`.
pub struct Toolkit<T: 'static> {
    adapter: wgpu::Adapter,
    el: EventLoop<T>,
    windows: Vec<Window>,
}

impl Toolkit<()> {
    /// Construct a new instance with default options.
    ///
    /// This chooses a low-power graphics adapter by preference.
    pub fn new() -> Self {
        Toolkit::<()>::new_custom(None)
    }
}

impl<T> Toolkit<T> {
    /// Construct an instance with custom options
    ///
    /// The graphics adapter is chosen according to the given options. If `None`
    /// is supplied, a low-power adapter will be chosen.
    ///
    /// The event loop supports user events of type `T`. Refer to winit's
    /// documentation of `EventLoop::with_user_event` for details.
    /// If not using user events, it may be necessary to force this type:
    /// ```
    /// let toolkit = kas_wgpu::Toolkit::<()>::new_custom(None);
    /// ```
    pub fn new_custom(adapter_options: Option<&wgpu::RequestAdapterOptions>) -> Self {
        let adapter_options = adapter_options.unwrap_or(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            backends: wgpu::BackendBit::PRIMARY,
        });
        let adapter = wgpu::Adapter::request(adapter_options).unwrap();

        Toolkit {
            adapter,
            el: EventLoop::with_user_event(),
            windows: vec![],
        }
    }

    /// Assume ownership of and display a window.
    ///
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    pub fn add<W: kas::Window + 'static>(&mut self, window: W) -> Result<(), OsError> {
        self.add_boxed(Box::new(window))
    }

    /// Add a boxed window directly
    pub fn add_boxed(&mut self, window: Box<dyn kas::Window>) -> Result<(), OsError> {
        let win = Window::new(&self.adapter, &self.el, window)?;
        self.windows.push(win);
        Ok(())
    }

    /// Run the main loop.
    pub fn run(self) -> ! {
        let mut el = event::Loop::new(self.windows);
        self.el
            .run(move |event, _, control_flow| el.handle(event, control_flow))
    }
}
