// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::event::EventMgr;
use kas::widget::{EditField, RowSplitter, TextButton};
use kas::{Events, Window};

#[derive(Clone, Debug)]
enum Message {
    Decr,
    Incr,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let panes = (0..2).map(|n| EditField::new(format!("Pane {}", n + 1)).with_multi_line(true));
    let panes = RowSplitter::<EditField>::new(panes.collect());

    let ui = kas::singleton! {
        #[widget{
            layout = column! [
                row! [
                    TextButton::new_msg("−", Message::Decr),
                    TextButton::new_msg("+", Message::Incr),
                ],
                self.panes,
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget] panes: RowSplitter<EditField> = panes,
        }
        impl Events for Self {
            type Data = ();

            fn handle_message(&mut self, mgr: &mut EventMgr) {
                if let Some(msg) = mgr.try_pop::<Message>() {
                    match msg {
                        Message::Decr => {
                            mgr.config_mgr(|mgr| self.panes.pop(mgr));
                        }
                        Message::Incr => {
                            let n = self.panes.len() + 1;
                            mgr.config_mgr(|mgr| self.panes.push(
                                mgr,
                                EditField::new(format!("Pane {n}")).with_multi_line(true)
                            ));
                        }
                    };
                }
            }
        }
    };
    let window = Window::new(ui, "Slitter panes");

    let theme = kas_wgpu::ShadedTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
