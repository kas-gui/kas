// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::event::EventMgr;
use kas::macros::impl_scope;
use kas::model::SharedRc;
use kas::widgets::view::SingleView;
use kas::widgets::TextButton;
use kas::{Widget, Window};

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    #[derive(Clone, Debug)]
    struct Increment(i32);

    impl_scope! {
        #[widget{
            layout = column: [
                align(center): self.counter,
                row: [
                    TextButton::new_msg("−", Increment(-1)),
                    TextButton::new_msg("+", Increment(1)),
                ],
            ];
        }]
        #[derive(Clone, Debug, Default)]
        struct Counter {
            core: widget_core!(),
            // SingleView embeds a shared value, here default-constructed to 0
            #[widget] counter: SingleView<SharedRc<i32>>,
        }
        impl Self {
            fn new(data: SharedRc<i32>) -> Self {
                Counter {
                    core: Default::default(),
                    counter: SingleView::new(data),
                }
            }
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(Increment(x)) = mgr.try_pop_msg() {
                    self.counter.update_value(mgr, |v| v + x);
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Counter" }
        }
    };

    let data = SharedRc::new(0);

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?
        .with(Counter::new(data.clone()))?
        .with(Counter::new(data))?
        .run()
}
