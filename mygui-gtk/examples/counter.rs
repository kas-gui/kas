//! Counter example (simple button)

#[macro_use]
extern crate mygui;
extern crate mygui_gtk;

use mygui::event::{NoResponse};
use mygui::widget::{
    canvas::Text,
    control::TextButton,
    window::SimpleWindow
};

use mygui::toolkit::Toolkit;
use mygui_gtk::{GtkToolkit, Error};

enum Message {
    None,
    Clicked,
}

impl From<NoResponse> for Message {
    fn from(_: NoResponse) -> Self {
        Message::None
    }
}

fn main() -> Result<(), Error> {
    let window = SimpleWindow::new(   // construct with default state and handler
        make_layout!(vertical<B[Message]>; self, tk, msg;
            display: Text = Text::from("0") => msg,
            button: B = TextButton::new("increment", || Message::Clicked) =>
                {
                    match msg {
                        Message::None => (),
                        Message::Clicked => {
                            self.counter += 1;
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
    toolkit.add(&window);
    toolkit.main();
    Ok(())
}
