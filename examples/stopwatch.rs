// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::class::HasString;
use kas::event::{ConfigMgr, Event, EventMgr, Response};
use kas::widget::{Frame, Label, TextButton};
use kas::{Decorations, Events, Widget, WidgetCore, WidgetExt, Window};

#[derive(Clone, Debug)]
struct MsgReset;
#[derive(Clone, Debug)]
struct MsgStart;

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Widget> {
    Box::new(kas::singleton! {
        #[widget{
            layout = row! [
                self.display,
                TextButton::new_msg("&reset", MsgReset),
                TextButton::new_msg("&start / &stop", MsgStart),
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget] display: impl Widget + HasString = Frame::new(Label::new("0.000".to_string())),
            saved: Duration,
            start: Option<Instant>,
        }
        impl Events for Self {
            fn configure(&mut self, mgr: &mut ConfigMgr) {
                mgr.enable_alt_bypass(self.id_ref(), true);
            }
            fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
                match event {
                    Event::TimerUpdate(0) => {
                        if let Some(start) = self.start {
                            let dur = self.saved + (Instant::now() - start);
                            let text = format!("{}.{:03}", dur.as_secs(), dur.subsec_millis());
                            *mgr |= self.display.set_string(text);
                            mgr.request_timer_update(self.id(), 0, Duration::new(0, 1), true);
                        }
                        Response::Used
                    }
                    _ => Response::Unused,
                }
            }
            fn handle_message(&mut self, mgr: &mut EventMgr) {
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
                        mgr.request_timer_update(self.id(), 0, Duration::new(0, 0), true);
                    }
                }
            }
        }
    })
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let window = Window::new_boxed(make_window(), "Stopwatch")
        .with_decorations(Decorations::Border)
        .with_transparent(true)
        .with_restrictions(true, true);

    let theme = kas_wgpu::ShadedTheme::new()
        .with_colours("dark")
        .with_font_size(18.0);
    kas::shell::DefaultShell::new(theme)?.with(window)?.run()
}
