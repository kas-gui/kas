//! GTK toolkit for mygui
//! 
//! Event handling

use gdk;
use gtk;

pub(crate) fn handler(event: &mut gdk::Event) {
    use gdk::EventType::*;
    match event.get_event_type() {
        Nothing => return,  // ignore this event
        
        // let GTK handle these for now:
        ButtonPress |
        ButtonRelease |
        ClientEvent |
        Configure |     // TODO: layout
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
}
