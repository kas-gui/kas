// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! GTK toolkit for kas
//! 
//! Event handling

use gdk::{Event};
use gtk;

use crate::{widget, window};

/// This function is registered as the GTK event handler, allowing event interception
pub(crate) fn handler(event: &mut Event) {
    use gdk::EventType::*;
    
    match event.get_event_type() {
        Nothing => return,  // ignore this event
        
        Configure => {
            #[cfg(feature = "layout")] {
                // TODO: use downcast_ref available in next GDK version
                window::with_list(|list| list.configure(event.clone().downcast().unwrap()));
                // TODO: emit expose event?
                return;
            }
            // else: fall through
        },
        
        _ => {
            // This hook can be used to trace events
            //println!("Event: {:?}", event);
        }
    }
    
    gtk::main_do_event(event);
    
    match event.get_event_type() {
        #[cfg(not(feature = "layout"))]
        Configure => {
            window::with_list(|list| list.post_configure(event))
        },
        
        _ => {}
    }
}

impl window::WindowList {
    #[cfg(feature = "layout")]
    fn configure(&mut self, event: gdk::EventConfigure) {
        let size = event.get_size();
        let size = (size.0 as i32, size.1 as i32);
        if let Some(gdk_win) = event.get_window() {
            self.for_gdk_win(gdk_win, |win, _gwin| {
                // TODO: this does some redundant work. Additionally the
                // algorithm is not optimal. Unfortunately we cannot
                // initialise constraints when constructing the widgets since
                // GTK does not give correct the size hints then.
                win.configure_widgets(self);
                win.resize(self, size);
            });
        }
    }
    
    #[cfg(not(feature = "layout"))]
    fn post_configure(&mut self, event: &Event) {
        if let Some(gdk_win) = event.get_window() {
            self.for_gdk_win(gdk_win, |win, _gwin| {
                win.sync_size(&widget::Toolkit);
            });
        }
    }
}
