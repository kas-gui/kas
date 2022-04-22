// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::layout;
use kas::macros::{make_layout, make_widget};
use kas::prelude::*;
use kas::widgets::{Label, TextButton, Window};

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    #[derive(Clone)]
    struct Increment(i32);

    let counter = make_widget! {
        #[widget]
        struct {
            #[widget]
            display: Label<String> = Label::from("0"),
            #[widget]
            b_decr = TextButton::new_msg("−", Increment(-1)),
            #[widget]
            b_incr = TextButton::new_msg("+", Increment(1)),
            count: i32 = 0,
        }
        impl Handler for Self {
            fn on_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(Increment(incr)) = mgr.try_pop_msg() {
                    self.count += incr;
                    *mgr |= self.display.set_string(self.count.to_string());
                }
            }
        }
        impl Layout for Self {
            fn layout<'a>(&'a mut self) -> layout::Layout<'a> {
                make_layout!(self.core;
                    column: [
                        align(center): self.display,
                        row: [self.b_decr, self.b_incr],
                    ]
                )
            }
        }
    };

    let window = Window::new("Counter", counter);

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
