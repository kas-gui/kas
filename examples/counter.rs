// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::macros::{make_widget, VoidMsg};
use kas::prelude::*;
use kas::widgets::{row, Label, TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let window = Window::new(
        "Counter",
        make_widget! {
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(halign=centre)] display: Label<String> = Label::from("0"),
                #[widget(use_msg = handle_button)] buttons -> Message = row![
                    TextButton::new_msg("−", Message::Decr),
                    TextButton::new_msg("+", Message::Incr),
                ],
                counter: usize = 0,
            }
            impl {
                fn handle_button(&mut self, mgr: &mut Manager, msg: Message) {
                    match msg {
                        Message::Decr => self.counter = self.counter.saturating_sub(1),
                        Message::Incr => self.counter = self.counter.saturating_add(1),
                    }
                    *mgr |= self.display.set_string(self.counter.to_string());
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
