// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Toolkit for kas

mod event;
mod widget;
mod window;
mod tkd;

use kas::Window;

use winit::event_loop::EventLoop;

use std::marker::PhantomData;
use std::{cell::RefCell, rc::Rc};


/// Builds a toolkit over a `winit::event_loop::EventLoop`.
pub struct Toolkit<T: 'static> {
    el: EventLoop<T>,
    windows: Vec<window::Window>,
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
    pub fn add<W: Window + 'static>(&mut self, window: W) where Self: Sized {
        self.add_rc(Rc::new(RefCell::new(window)))
    }
    
    /// Specialised version of `add`; typically toolkits only need to implement
    /// this.
    pub fn add_rc(&mut self, win: Rc<RefCell<dyn kas::Window>>) {
//         windows.push_back(
    }
    
    /// Run the main loop.
    pub fn run(mut self) -> ! {
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
