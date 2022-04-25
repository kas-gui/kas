// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::event::{EventMgr, Handler};
use kas::macros::make_widget;
use kas::widgets::{EditField, RowSplitter, TextButton, Window};

#[derive(Clone, Debug)]
enum Message {
    Decr,
    Incr,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let buttons = make_widget! {
        #[widget{
            layout = row: *;
        }]
        struct {
            #[widget] _ = TextButton::new_msg("−", Message::Decr),
            #[widget] _ = TextButton::new_msg("+", Message::Incr),
        }
    };
    let panes = (0..2).map(|n| EditField::new(format!("Pane {}", n + 1)).multi_line(true));
    let panes = RowSplitter::<EditField>::new(panes.collect());

    let window = Window::new(
        "Slitter panes",
        make_widget! {
            // TODO: use vertical splitter
            #[widget{
                layout = column: *;
            }]
            struct {
                #[widget] _ = buttons,
                #[widget] panes: RowSplitter<EditField> = panes,
            }
            impl Handler for Self {
                fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                    if let Some(msg) = mgr.try_pop_msg::<Message>() {
                        match msg {
                            Message::Decr => {
                                mgr.set_rect_mgr(|mgr| self.panes.pop(mgr));
                            }
                            Message::Incr => {
                                let n = self.panes.len() + 1;
                                mgr.set_rect_mgr(|mgr| self.panes.push(
                                    mgr,
                                    EditField::new(format!("Pane {}", n)).multi_line(true)
                                ));
                            }
                        };
                    }
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
