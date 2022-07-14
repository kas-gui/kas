// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::macros::impl_scope;
use kas::model::SharedRc;
use kas::widgets::view::{driver, SingleView};
use kas::Window;

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    impl_scope! {
        #[widget{
            layout = self.spinner;
        }]
        #[derive(Debug)]
        struct Counter {
            core: widget_core!(),
            // SingleView embeds a shared value, here default-constructed to 0
            #[widget] spinner: SingleView<SharedRc<i32>, driver::Spinner<i32>>,
        }
        impl Self {
            fn new(data: SharedRc<i32>) -> Self {
                let driver = driver::Spinner::new(i32::MIN..=i32::MAX, 1);
                Counter {
                    core: Default::default(),
                    spinner: SingleView::new_with_driver(driver, data),
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
