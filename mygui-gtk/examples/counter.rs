//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use mygui::control::TextButton;
use mygui::display::Text;
use mygui::event::NoResponse;
use mygui::macros::make_widget;
use mygui::window::SimpleWindow;

use mygui::toolkit::Toolkit;
use mygui_gtk::{Error, GtkToolkit};

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
    let buttons = make_widget!(horizontal => Message;
        #[widget] _ = TextButton::new("−", || Message::Decr),
        #[widget] _ = TextButton::new("+", || Message::Incr),
    );
    let window = SimpleWindow::new(
        make_widget!(vertical => NoResponse;
            #[widget] display: Text = Text::from("0"),
            #[widget] buttons -> Message = buttons =>
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
            },
            counter: usize = 0
        ),
    );

    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
