//! Hello world example

extern crate kas;
extern crate kas_gtk;

use kas::display::Text;
use kas::dialog::{action_close, MessageBox};

use kas::Toolkit;

fn main() -> Result<(), kas_gtk::Error> {
    // Build widgets.
    // Message is a Window with an "Ok" button and notification status.
    // Each Window::new method creates objects then solves constraints.
    let window = MessageBox::new(
        /*Notify::Info,*/
        Text::from("Hello world"),
        action_close,
    );

    let mut toolkit = kas_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
