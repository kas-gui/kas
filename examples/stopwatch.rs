// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::class::HasString;
use kas::event::{ConfigCx, Event, EventCx, Response};
use kas::widget::{Frame, Label, TextButton};
use kas::{Decorations, Widget, WidgetCore, WidgetExt, Window};

#[derive(Clone, Debug)]
struct MsgReset;
#[derive(Clone, Debug)]
struct MsgStart;

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Window> {
    Box::new(kas::singleton! {
        #[widget{
            layout = row: [
                self.display,
                TextButton::new_msg("&reset", MsgReset),
                TextButton::new_msg("&start / &stop", MsgStart),
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] display: impl Widget + HasString = Frame::new(Label::new("0.000".to_string())),
            saved: Duration,
            start: Option<Instant>,
        }
        impl Widget for Self {
            fn configure(&mut self, mgr: &mut ConfigCx<Self::Data>) {
                mgr.enable_alt_bypass(self.id_ref(), true);
            }
            fn handle_event(&mut self, mgr: &mut EventCx<Self::Data>, event: Event) -> Response {
                match event {
                    Event::TimerUpdate(0) => {
                        if let Some(start) = self.start {
                            let dur = self.saved + (Instant::now() - start);
                            let text = format!("{}.{:03}", dur.as_secs(), dur.subsec_millis());
                            *mgr |= self.display.set_string(text);
                            mgr.request_update(self.id(), 0, Duration::new(0, 1), true);
                        }
                        Response::Used
                    }
                    _ => Response::Unused,
                }
            }
            fn handle_message(&mut self, mgr: &mut EventCx<Self::Data>) {
                if let Some(MsgReset) = mgr.try_pop() {
                    self.saved = Duration::default();
                    self.start = None;
                    *mgr |= self.display.set_str("0.000");
                } else if let Some(MsgStart) = mgr.try_pop() {
                    if let Some(start) = self.start {
                        self.saved += Instant::now() - start;
                        self.start = None;
                    } else {
                        self.start = Some(Instant::now());
                        mgr.request_update(self.id(), 0, Duration::new(0, 0), true);
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Stopwatch" }
            fn decorations(&self) -> Decorations {
                Decorations::Border
            }
            fn transparent(&self) -> bool {
                true
            }
            fn restrict_dimensions(&self) -> (bool, bool) {
                (true, true)
            }
        }
    })
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas_wgpu::ShadedTheme::new()
        .with_colours("dark")
        .with_font_size(18.0);
    kas::shell::DefaultShell::new(theme)?
        .with_boxed(make_window())?
        .run()
}
