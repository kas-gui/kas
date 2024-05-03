// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::decorations::Decorations;
use kas::prelude::*;
use kas::widgets::{format_data, row, Adapt, Button};

#[derive(Clone, Debug)]
struct MsgReset;
#[derive(Clone, Debug)]
struct MsgStart;

#[derive(Debug, Default)]
struct Timer {
    elapsed: Duration,
    last: Option<Instant>,
}

fn make_window() -> impl Widget<Data = ()> {
    let ui = row![
        format_data!(timer: &Timer, "{}.{:03}", timer.elapsed.as_secs(), timer.elapsed.subsec_millis()),
        Button::label_msg("&reset", MsgReset).map_any(),
        Button::label_msg("&start / &stop", MsgStart).map_any(),
    ];

    Adapt::new(ui, Timer::default())
        .on_configure(|cx, _| cx.enable_alt_bypass(true))
        .on_message(|_, timer, MsgReset| *timer = Timer::default())
        .on_message(|cx, timer, MsgStart| {
            let now = Instant::now();
            if let Some(last) = timer.last.take() {
                timer.elapsed += now - last;
            } else {
                timer.last = Some(now);
                cx.request_timer(0, Duration::new(0, 0));
            }
        })
        .on_timer(0, |cx, timer, _| {
            if let Some(last) = timer.last {
                let now = Instant::now();
                timer.elapsed += now - last;
                timer.last = Some(now);
                cx.request_timer(0, Duration::new(0, 1));
            }
        })
}

fn main() -> kas::app::Result<()> {
    env_logger::init();

    let window = Window::new(make_window(), "Stopwatch")
        .with_decorations(Decorations::Border)
        .with_transparent(true)
        .with_restrictions(true, true);

    let theme = kas_wgpu::ShadedTheme::new();
    let mut app = kas::app::Default::with_theme(theme).build(())?;
    let _ = app.config_mut().font.set_size(24.0);
    let _ = app.config_mut().theme.set_active_scheme("dark");
    app.with(window).run()
}
