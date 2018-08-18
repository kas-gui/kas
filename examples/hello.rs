//! Hello world example

extern crate mygui;

use mygui::canvas::Text;
use mygui::window::message_box;

fn main() {
    // Build widgets.
    // Message is a Window with an "Ok" button and notification status.
    // Each Window::new method creates objects then solves constraints.
    let mut window = message_box(/*Notify::Info,*/ Text::from("Hello world"));
    
    window.display();
}
