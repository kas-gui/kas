// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use kas::prelude::*;
use kas::widget::{Adapt, Button, EditField, Splitter};

#[derive(Clone, Debug)]
enum Message {
    Decr,
    Incr,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let ui = kas::column![
        kas::row![
            Button::label_msg("−", Message::Decr),
            Button::label_msg("+", Message::Incr),
        ]
        .map_any(),
        Splitter::right([]).on_update(|cx, panes, len| panes.resize_with(len, cx, *len, |n| {
            EditField::text(format!("Pane {}", n + 1)).with_multi_line(true)
        })),
    ];

    let adapt = Adapt::new(ui, 3).on_message(|_, len, msg| {
        *len = match msg {
            Message::Decr => len.saturating_sub(1),
            Message::Incr => len.saturating_add(1),
        }
    });

    let window = Window::new(adapt, "Slitter panes");

    let theme = kas_wgpu::ShadedTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
