// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple text editor

use kas::prelude::*;
use kas::widgets::{EditBox, EditGuard};

struct Guard;
impl EditGuard for Guard {
    type Data = ();
}

fn editor() -> impl Widget<Data = ()> {
    EditBox::new(Guard)
        .with_multi_line(true)
        .with_lines(5.0, 20.0)
        .with_width_em(10.0, 30.0)
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let theme = kas::theme::FlatTheme::new();
    let app = kas::runner::Runner::with_theme(theme).build(())?;
    let window = Window::new(editor(), "Editor");
    app.with(window).run()
}
