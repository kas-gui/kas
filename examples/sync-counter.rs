// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::model::SharedRc;
use kas::view::{driver, SingleView};
use kas::widgets::dialog::Window;

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let driver = driver::Spinner::new(i32::MIN..=i32::MAX, 1);
    let c1 = SingleView::new_with_driver(driver, SharedRc::new(0));
    let c2 = SingleView::new_with_driver(driver, c1.data().clone());

    let theme = kas::theme::ShadedTheme::new().with_font_size(24.0);
    kas::shell::Toolkit::new(theme)?
        .with(Window::new("Counter 1", c1))?
        .with(Window::new("Counter 2", c2))?
        .run()
}
