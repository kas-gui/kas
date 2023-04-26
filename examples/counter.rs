// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::prelude::*;
use kas::widget::dialog::Window;
use kas::widget::{button, format_text, Adapt, BoxColumn, Row};

#[derive(Clone, Debug)]
struct Increment(i32);

fn counter() -> impl Widget<Data = ()> {
    // TODO: column, row macros?
    // TODO: auto-boxing? Generic Button?
    let tree = BoxColumn::new_vec(vec![
        Box::new(format_text!(count, "{}", count)),
        Box::new(Row::new_vec(vec![
            button("−", Increment(-1)),
            button("+", Increment(1)),
        ])),
    ]);

    Adapt::new(tree, 0, |_, count| count).on_message(|_, count, Increment(add)| *count += add)
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas::theme::SimpleTheme::new().with_font_size(24.0);
    kas::shell::DefaultShell::new(theme)?
        .with(Window::new("Counter", counter()))?
        .run()
}
