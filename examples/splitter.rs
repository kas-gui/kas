// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::prelude::*;
use kas::widget::{button, dialog, Adapt, EditField, RowSplitter};

#[derive(Clone, Debug)]
enum Message {
    Decr,
    Incr,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    fn make_pane(n: usize) -> EditField<usize> {
        EditField::new(format!("Pane {}", n + 1)).with_multi_line(true)
    }

    // TODO: add on_init method and use to construct initial panes
    let panes = (0..2).map(make_pane);
    let len = panes.len();

    let ui = kas::column![
        row![button("−", Message::Decr), button("+", Message::Incr),],
        RowSplitter::new(panes.collect()).on_update(|panes, cx| panes.resize_with(
            cx,
            *cx.data(),
            make_pane
        )),
    ];

    let adapt = Adapt::new(ui, len, |_, len| len).on_message(|_, len, msg| {
        *len = match msg {
            Message::Decr => len.saturating_sub(1),
            Message::Incr => len.saturating_add(1),
        }
    });

    let window = dialog::Window::new("Slitter panes", adapt);

    let theme = kas_wgpu::ShadedTheme::new();
    kas::shell::DefaultShell::new(theme)?.with(window)?.run()
}
