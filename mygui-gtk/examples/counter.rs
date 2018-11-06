//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use mygui::control::TextButton;
use mygui::display::Text;
use mygui::macros::make_widget;
use mygui::event::{NoResponse};
use mygui::window::SimpleWindow;

use mygui::toolkit::Toolkit;
use mygui_gtk::{GtkToolkit, Error};

enum Message {
    None,
    Decr,
    Incr,
}

impl From<NoResponse> for Message {
    fn from(_: NoResponse) -> Self {
        Message::None
    }
}

fn main() -> Result<(), Error> {
    let window = SimpleWindow::new(   // construct with default state and handler
        make_widget!(vertical;
            display: Text = Text::from("0"),
            buttons: [Message] = make_widget!(
                horizontal;
                decr = TextButton::new("−", || Message::Decr),
                incr = TextButton::new("+", || Message::Incr);;
                Message) =>
            {
                match msg {
                    Message::None => (),
                    Message::Decr => {
                        self.counter = self.counter.saturating_sub(1);
                        self.display.set_text(tk, &self.counter.to_string());
                    }
                    Message::Incr => {
                        self.counter = self.counter.saturating_add(1);
                        self.display.set_text(tk, &self.counter.to_string());
                    }
                };
                NoResponse::None
            };
            counter: usize = 0;
            NoResponse
        )
    );
    
    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
