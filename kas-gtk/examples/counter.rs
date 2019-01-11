//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use kas::control::TextButton;
use kas::display::Text;
use kas::event::NoResponse;
use kas::macros::{NoResponse, make_widget};
use kas::{SimpleWindow, Toolkit, TkWidget};

#[derive(Debug, NoResponse)]
enum Message {
    None,
    Decr,
    Incr,
}

fn main() -> Result<(), kas_gtk::Error> {
    let buttons = make_widget!(
        horizontal => Message;
        struct {
            #[widget] _ = TextButton::new("−", || Message::Decr),
            #[widget] _ = TextButton::new("+", || Message::Incr),
        }
    );
    let window = SimpleWindow::new(make_widget!(
        vertical => NoResponse;
        struct {
            #[widget] display: Text = Text::from("0"),
            #[widget(handler = handle_button)] buttons -> Message = buttons,
            counter: usize = 0,
        }
        impl {
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
        }));

    let mut toolkit = kas_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
