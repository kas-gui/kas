// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::class::HasString;
use kas::event::{Event, EventMgr, Response};
use kas::layout::SetRectMgr;
use kas::macros::impl_singleton;
use kas::widgets::{Frame, Label, TextButton, Window};
use kas::{Widget, WidgetCore, WidgetExt};

#[derive(Clone, Debug)]
struct MsgReset;
#[derive(Clone, Debug)]
struct MsgStart;

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Window> {
    // Construct a row widget, with state and children
    let stopwatch = impl_singleton! {
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
            #[widget] display: impl HasString = Frame::new(Label::new("0.000".to_string())),
            saved: Duration,
            start: Option<Instant>,
        }
        impl Widget for Self {
            fn configure(&mut self, mgr: &mut SetRectMgr) {
                mgr.enable_alt_bypass(self.id_ref(), true);
            }
            fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
                match event {
                    Event::TimerUpdate(0) => {
                        if let Some(start) = self.start {
                            let dur = self.saved + (Instant::now() - start);
                            let text = format!("{}.{:03}", dur.as_secs(), dur.subsec_millis());
                            *mgr |= self.display.set_string(text);
                            mgr.update_on_timer(Duration::new(0, 1), self.id(), 0);
                        }
                        Response::Used
                    }
                    _ => Response::Unused,
                }
            }
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(MsgReset) = mgr.try_pop_msg() {
                    self.saved = Duration::default();
                    self.start = None;
                    *mgr |= self.display.set_str("0.000");
                } else if let Some(MsgStart) = mgr.try_pop_msg() {
                    if let Some(start) = self.start {
                        self.saved += Instant::now() - start;
                        self.start = None;
                    } else {
                        self.start = Some(Instant::now());
                        mgr.update_on_timer(Duration::new(0, 0), self.id(), 0);
                    }
                }
            }
        }
    };

    let mut window = Window::new("Stopwatch", stopwatch);
    window.set_restrict_dimensions(true, true);
    Box::new(window)
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas::theme::ShadedTheme::new()
        .with_colours("dark")
        .with_font_size(18.0);
    kas::shell::Toolkit::new(theme)?
        .with_boxed(make_window())?
        .run()
}
