// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

mod event;
mod widget;
mod window;
mod tkd;

use rgx::core::*;
use raw_window_handle::HasRawWindowHandle;

use std::marker::PhantomData;
use std::{cell::RefCell, rc::Rc};


/// Builds a toolkit over a `winit::Window`
pub struct Toolkit {
    rend: Renderer,
    windows: Vec<window::Window>,
}

impl Toolkit {
    /// Construct a new instance.
    pub fn new(window: &winit::window::Window) -> Self {
        let rend = Renderer::new(window.raw_window_handle());
        
        Toolkit {
            rend,
            windows: vec![],
        }
    }
}

impl kas::Toolkit for Toolkit {
    fn add_rc(&mut self, win: Rc<RefCell<dyn kas::Window>>) {
//         windows.push_back(
    }
    
    fn run(mut self) -> ! {
//         window::with_list(|list| {
//             for window in &list.windows {
//                 window.win.borrow_mut().on_start(&widget::Toolkit);
//             }
//         });
        
        let event_loop = winit::event_loop::EventLoop::new();
        event_loop.run(move |event, _, control_flow| {
            self.handler(event, control_flow)
        })
    }
}
