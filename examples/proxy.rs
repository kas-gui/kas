// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Asynchronous events using a proxy
//!
//! This is a copy-cat of Druid's async event example, demonstrating usage of
//! `Runner::create_proxy()`. For a more integrated approach to async, see
//! `EventState::push_async()` and `push_spawn()`.

use std::thread;
use std::time::{Duration, Instant};

use kas::draw::color::Rgba;
use kas::prelude::*;
use kas::theme::{Text, TextClass};

#[derive(Debug)]
struct SetColor(Rgba);

struct AppData {
    color: Option<Rgba>,
}

impl kas::runner::AppData for AppData {
    fn handle_messages(&mut self, messages: &mut kas::messages::MessageStack) {
        if let Some(SetColor(color)) = messages.try_pop() {
            self.color = Some(color);
        }
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let data = AppData { color: None };
    let app = kas::runner::Default::new(data)?;

    // We construct a proxy from the app to enable cross-thread communication.
    let proxy = app.create_proxy();
    thread::spawn(move || generate_colors(proxy));

    let widget = ColourSquare::new();
    let window = Window::new(widget, "Async event demo");

    app.with(window).run()
}

impl_scope! {
    // A custom widget incorporating "Loading..." text, drawing and layout.
    #[widget]
    struct ColourSquare {
        core: widget_core!(),
        color: Option<Rgba>,
        loading_text: Text<&'static str>,
    }
    impl Self {
        fn new() -> Self {
            ColourSquare {
                core: Default::default(),
                color: None,
                loading_text: Text::new("Loading...", TextClass::Label(false)),
            }
        }
    }
    impl Layout for ColourSquare {
        fn size_rules(&mut self, sizer: SizeCx, _axis: AxisInfo) -> SizeRules {
            SizeRules::fixed_scaled(100.0, 10.0, sizer.scale_factor())
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.loading_text.set_rect(cx, rect, hints.combine(AlignHints::CENTER));
        }

        fn draw(&mut self, mut draw: DrawCx) {
            if let Some(color) = self.color {
                let draw = draw.draw_device();
                draw.rect((self.rect()).cast(), color);
            } else {
                draw.text(self.rect(), &self.loading_text);
            }
        }
    }
    impl Events for ColourSquare {
        type Data = AppData;

        fn configure(&mut self, cx: &mut ConfigCx) {
            self.loading_text.set_align((Align::Center, Align::Center));
            cx.text_configure(&mut self.loading_text);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &AppData) {
            self.color = data.color;
            cx.redraw(self);
        }
    }
}

fn generate_colors(mut proxy: kas::runner::Proxy) {
    // Loading takes time:
    thread::sleep(Duration::from_secs(1));

    // This function is called in a separate thread, and runs until the program ends.
    let start_time = Instant::now();

    loop {
        let hue = (Instant::now() - start_time).as_secs_f32() / 5.0;

        // convert from HSV, using S=V=1 (see Wikipedia):
        let f = |n| {
            let k: f32 = (n + hue * 6.0) % 6.0;
            1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
        };
        let c = Rgba::rgb(f(5.0), f(3.0), f(1.0));

        if proxy.push(SetColor(c)).is_err() {
            // Sending failed; we should quit
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }
}
