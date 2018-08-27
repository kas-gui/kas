//! Hello world example

extern crate mygui;

use mygui::widget::canvas::Text;
use mygui::widget::window::{MessageBox, action_close};

fn main() {
    // Build widgets.
    // Message is a Window with an "Ok" button and notification status.
    // Each Window::new method creates objects then solves constraints.
    let mut window = MessageBox::new(/*Notify::Info,*/
        Text::from("Hello world"),
        action_close);
    
    window.display();
}
