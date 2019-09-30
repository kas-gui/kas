// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling

use winit::event::Event;
use winit::event_loop::ControlFlow;

use crate::{Toolkit, widget, window};

impl Toolkit {
    #[inline]
    pub(crate) fn handler(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        unimplemented!()
        /*
        use gdk::EventType::*;
        
        match event.get_event_type() {
            Nothing => return,  // ignore this event
            
            Configure => {
                window::with_list(|list| list.configure(event.clone().downcast().unwrap()));
                return;
            },
            
            _ => {
                // This hook can be used to trace events
                //println!("Event: {:?}", event);
            }
        }
        */
    }
}

// impl window::WindowList {
//     fn configure(&mut self/*, event: gdk::EventConfigure*/) {
//         unimplemented!()
//         /*
//         let size = event.get_size();
//         let size = (size.0 as i32, size.1 as i32);
//         if let Some(gdk_win) = event.get_window() {
//             self.for_gdk_win(gdk_win, |win, _gwin| {
//                 // TODO: this does some redundant work. Additionally the
//                 // algorithm is not optimal. Unfortunately we cannot
//                 // initialise constraints when constructing the widgets since
//                 // GTK does not give correct the size hints then.
//                 win.configure_widgets(self);
//                 win.resize(self, size);
//             });
//         }
//         */
//     }
// }
