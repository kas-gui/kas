// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)
#![feature(proc_macro_hygiene)]

use kas::event::{Manager, VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Label, RowSplitter, TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[layout(row)]
        #[handler(msg = Message)]
        struct {
            #[widget] _ = TextButton::new("−", Message::Decr),
            #[widget] _ = TextButton::new("+", Message::Incr),
        }
    };
    let mut panes = RowSplitter::<Label>::default();
    let _ = panes.resize_with(2, |n| Label::new(format!("Pane {}", n)));

    let window = Window::new(
        "Slitter panes",
        make_widget! {
            // TODO: use vertical splitter
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(handler = handle_button)] buttons -> Message = buttons,
                #[widget] panes: RowSplitter<Label> = panes,
                counter: usize = 0,
            }
            impl {
                fn handle_button(&mut self, mgr: &mut Manager, msg: Message)
                    -> VoidResponse
                {
                    match msg {
                        Message::Decr => {
                            *mgr += self.panes.pop().1;
                        }
                        Message::Incr => {
                            let n = self.panes.len() + 1;
                            *mgr += self.panes.push(Label::new(format!("Pane {}", n)));
                        }
                    };
                    VoidResponse::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
