// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::event::{EventMgr, Handler};
use kas::macros::make_widget;
use kas::updatable::SharedRc;
use kas::widgets::view::SingleView;
use kas::widgets::{TextButton, Window};

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    #[derive(Clone, Debug)]
    struct Increment(i32);

    let window = Window::new(
        "Counter",
        make_widget! {
            #[derive(Clone)]
            #[widget{
                layout = column: [
                    align(center): self.counter,
                    row: [self.b_decr, self.b_incr],
                ];
            }]
            struct {
                // SingleView embeds a shared value, here default-constructed to 0
                #[widget] counter: SingleView<SharedRc<i32>> = Default::default(),
                #[widget] b_decr = TextButton::new_msg("−", Increment(-1)),
                #[widget] b_incr = TextButton::new_msg("+", Increment(1)),
            }
            impl Handler for Self {
                fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                    if let Some(Increment(x)) = mgr.try_pop_msg() {
                        self.counter.update_value(mgr, |v| v + x);
                    }
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
