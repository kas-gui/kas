// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::event::{EventMgr, VoidMsg};
use kas::macros::{make_widget, VoidMsg};
use kas::widgets::{EditField, RowSplitter, TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[widget{
            layout = row: *;
        }]
        #[handler(msg = Message)]
        struct {
            #[widget] _ = TextButton::new_msg("−", Message::Decr),
            #[widget] _ = TextButton::new_msg("+", Message::Incr),
        }
    };
    let mut panes = RowSplitter::<EditField>::default();
    let _ = panes.resize_with(2, |n| {
        EditField::new(format!("Pane {}", n + 1)).multi_line(true)
    });

    let window = Window::new(
        "Slitter panes",
        make_widget! {
            // TODO: use vertical splitter
            #[widget{
                layout = column: *;
            }]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(use_msg = handle_button)] buttons -> Message = buttons,
                #[widget] panes: RowSplitter<EditField> = panes,
            }
            impl Self {
                fn handle_button(&mut self, mgr: &mut EventMgr, msg: Message) {
                    match msg {
                        Message::Decr => {
                            *mgr |= self.panes.pop().1;
                        }
                        Message::Incr => {
                            let n = self.panes.len() + 1;
                            *mgr |= self.panes.push(EditField::new(format!("Pane {}", n)).multi_line(true));
                        }
                    };
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
