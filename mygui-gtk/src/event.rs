//! GTK toolkit for mygui
//! 
//! Event handling

use gdk::{Event, EventConfigure};
use gtk;

use {GtkToolkit, for_toolkit};

// If true, use mygui's layout configuration, otherwise use GTK's.
const MYGUI_LAYOUT: bool = false;

pub(crate) fn handler(event: &mut Event) {
    use gdk::EventType::*;
    
    match event.get_event_type() {
        Nothing => return,  // ignore this event
        
        Configure if MYGUI_LAYOUT => {
            // TODO: use downcast_ref available in next GDK version
            for_toolkit(|tk| tk.configure(event.clone().downcast().unwrap()));
            // TODO: emit expose event?
            return;
        },
        
        // let GTK handle these for now:
        ButtonPress |
        ButtonRelease |
        ClientEvent |
        Configure |
        Damage |
        Delete |
        DoubleButtonPress |
        EnterNotify |
        Expose |
        FocusChange |
        GrabBroken |
        KeyPress |
        KeyRelease |
        LeaveNotify |
        Map |
        MotionNotify |
        PropertyNotify |
        SelectionClear |
        SelectionNotify |
        SelectionRequest |
        Setting |
        TripleButtonPress |
        Unmap |
        VisibilityNotify |
        WindowState => {
            // fall through
        },
        
        _ => {
            println!("Event: {:?}", event);
        }
    }
    
    gtk::main_do_event(event);
    
    match event.get_event_type() {
        Configure if !MYGUI_LAYOUT => {
            for_toolkit(|tk| tk.post_configure(event))
        },
        
        _ => {}
    }
}

impl GtkToolkit {
    fn configure(&self, event: EventConfigure) {
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
    
    fn post_configure(&self, event: &Event) {
        if let Some(gdk_win) = event.get_window() {
            self.for_gdk_win(gdk_win, |win, _gwin| {
                win.sync_size(self);
            });
        }
    }
}
