//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use mygui::control::TextButton;
use mygui::display::Text;
use mygui::event::NoResponse;
use mygui::macros::{NoResponse, make_widget};
use mygui::{SimpleWindow, Toolkit, TkWidget};

#[derive(Debug, NoResponse)]
enum Message {
    None,
    Decr,
    Incr,
}

fn main() -> Result<(), mygui_gtk::Error> {
    let buttons = make_widget!(horizontal => Message;
        #[widget] _ = TextButton::new("−", || Message::Decr),
        #[widget] _ = TextButton::new("+", || Message::Incr);
    );
    let window = SimpleWindow::new(make_widget!(vertical => NoResponse;
            #[widget] display: Text = Text::from("0"),
            #[widget(handler = handle_button)] buttons -> Message = buttons,
            counter: usize = 0;
            fn handle_button(&mut self, tk: &TkWidget, msg: Message) -> NoResponse {
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
                NoResponse
            }
        ));

    let mut toolkit = mygui_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
