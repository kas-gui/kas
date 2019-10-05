// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

mod event;
mod widget;
mod window;

pub use window::Window;

use winit::event_loop::EventLoop;
use winit::error::OsError;





/// Builds a toolkit over a `winit::event_loop::EventLoop`.
pub struct Toolkit<T: 'static> {
    el: EventLoop<T>,
    windows: Vec<Window>,
}

impl Toolkit<()> {
    /// Construct a new instance.
    pub fn new() -> Self {
        Toolkit {
            el: EventLoop::new(),
            windows: vec![],
        }
    }
}

impl<T> Toolkit<T> {
    /// Construct an instance with given user event type
    /// 
    /// Refer to the winit's `EventLoop` documentation.
    pub fn with_user_event() -> Self {
        Toolkit {
            el: EventLoop::with_user_event(),
            windows: vec![],
        }
    }
    
    
    /// Assume ownership of and display a window.
    /// 
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    pub fn add<W: kas::Window + 'static>(&mut self, window: W)
        -> Result<(), OsError>
    {
        self.add_boxed(Box::new(window))
    }
    
    /// Add a boxed window directly
    pub fn add_boxed(&mut self, window: Box<dyn kas::Window>)
        -> Result<(), OsError>
    {
        let num0 = self.windows.last().map(|w| w.nums().1).unwrap_or(0);
        let win = Window::new(&self.el, window, num0)?;
        self.windows.push(win);
        Ok(())
    }
    
    /// Run the main loop.
    pub fn run(self) -> ! {
        let mut windows = self.windows;
        
        for window in windows.iter_mut() {
            window.prepare();
        }
        
        self.el.run(move |event, _, control_flow| {
            event::handler(&mut windows, event, control_flow)
        })
    }
}
