// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::event::{ConfigCx, Event, EventCx, IsUsed, Unused, Used};
use kas::widgets::{format_data, Button};
use kas::{Decorations, Events, Layout, LayoutExt, Widget, Window};

#[derive(Clone, Debug)]
struct MsgReset;
#[derive(Clone, Debug)]
struct MsgStart;

fn make_window() -> Box<dyn kas::Widget<Data = ()>> {
    Box::new(kas::impl_anon! {
        #[widget{
            layout = row! [
                self.display,
                Button::label_msg("&reset", MsgReset),
                Button::label_msg("&start / &stop", MsgStart),
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget(&self.elapsed)] display: impl Widget<Data = Duration> =
                format_data!(dur: &Duration, "{}.{:03}", dur.as_secs(), dur.subsec_millis()),
            last: Option<Instant>,
            elapsed: Duration,
        }
        impl Events for Self {
            type Data = ();

            fn configure(&mut self, cx: &mut ConfigCx) {
                cx.enable_alt_bypass(self.id_ref(), true);
            }
            fn handle_event(&mut self, cx: &mut EventCx, data: &(), event: Event) -> IsUsed {
                match event {
                    Event::Timer(0) => {
                        if let Some(last) = self.last {
                            let now = Instant::now();
                            self.elapsed += now - last;
                            self.last = Some(now);
                            cx.update(self.as_node(data));
                            cx.request_timer(self.id(), 0, Duration::new(0, 1));
                        }
                        Used
                    }
                    _ => Unused,
                }
            }
            fn handle_messages(&mut self, cx: &mut EventCx, data: &()) {
                if let Some(MsgReset) = cx.try_pop() {
                    self.elapsed = Duration::default();
                    self.last = None;
                    cx.update(self.as_node(data));
                } else if let Some(MsgStart) = cx.try_pop() {
                    let now = Instant::now();
                    if let Some(last) = self.last.take() {
                        self.elapsed += now - last;
                    } else {
                        self.last = Some(now);
                        cx.request_timer(self.id(), 0, Duration::new(0, 0));
                    }
                }
            }
        }
    })
}

fn main() -> kas::app::Result<()> {
    env_logger::init();

    let window = Window::new_boxed(make_window(), "Stopwatch")
        .with_decorations(Decorations::Border)
        .with_transparent(true)
        .with_restrictions(true, true);

    let theme = kas_wgpu::ShadedTheme::new()
        .with_colours("dark")
        .with_font_size(18.0);
    kas::app::Default::with_theme(theme)
        .build(())?
        .with(window)
        .run()
}
