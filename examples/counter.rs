// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::prelude::*;
use kas::widgets::{format_value, Adapt, Button};
use kas::{column, row};

#[derive(Clone, Debug)]
struct Increment(i32);

fn counter() -> impl Widget<Data = ()> {
    let tree = column![
        format_value!("{}").align(AlignHints::CENTER),
        row![
            Button::label_msg("−", Increment(-1)),
            Button::label_msg("+", Increment(1)),
        ]
        .map_any(),
    ];

    Adapt::new(tree, 0).on_message(|_, count, Increment(add)| *count += add)
}

fn main() -> kas::app::Result<()> {
    env_logger::init();

    let theme = kas::theme::SimpleTheme::new().with_font_size(24.0);
    kas::app::Default::with_theme(theme)
        .build(())?
        .with(Window::new(counter(), "Counter"))
        .run()
}
