//! Hello world example

extern crate mygui;
extern crate mygui_gtk;

use mygui::display::Text;
use mygui::window::{action_close, MessageBox};

use mygui::toolkit::Toolkit;
use mygui_gtk::{Error, GtkToolkit};

fn main() -> Result<(), Error> {
    // Build widgets.
    // Message is a Window with an "Ok" button and notification status.
    // Each Window::new method creates objects then solves constraints.
    let window = MessageBox::new(
        /*Notify::Info,*/
        Text::from("Hello world"),
        action_close,
    );

    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
