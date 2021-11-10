// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::event::{Manager, VoidMsg};
use kas::macros::make_widget;
use kas::updatable::SharedRc;
use kas::widgets::view::SingleView;
use kas::widgets::{row, TextButton, Window};

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let window = Window::new(
        "Counter",
        make_widget! {
            #[layout(column)]
            #[derive(Clone)]
            #[handler(msg = VoidMsg)]
            struct {
                // SingleView embeds a shared value, here default-constructed to 0
                #[widget(halign=centre)] counter: SingleView<SharedRc<i32>> = Default::default(),
                #[widget(use_msg = update)] buttons -> i32 = row![
                    TextButton::new_msg("−", -1),
                    TextButton::new_msg("+", 1),
                ],
            }
            impl Self {
                fn update(&mut self, mgr: &mut Manager, msg: i32) {
                    self.counter.update_value(mgr, |v| v + msg);
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?
        .with(window.clone())?
        .with(window)?
        .run()
}
