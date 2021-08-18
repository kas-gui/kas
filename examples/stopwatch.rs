// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)

use std::time::{Duration, Instant};

use kas::class::HasString;
use kas::event::{Event, Handler, Manager, Response, VoidMsg};
use kas::macros::make_widget;
use kas::widgets::{Frame, Label, TextButton, Window};
use kas::WidgetCore;

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Window> {
    // Construct a row widget, with state and children
    let stopwatch = make_widget! {
        #[layout(row)]
        #[widget(config=noauto)]
        struct {
            #[widget] display: impl HasString = Frame::new(Label::new("0.000".to_string())),
            #[widget(handler = reset)] _ = TextButton::new_msg("&reset", ()),
            #[widget(handler = start)] _ = TextButton::new_msg("&start / &stop", ()),
            saved: Duration = Duration::default(),
            start: Option<Instant> = None,
        }
        impl {
            fn reset(&mut self, mgr: &mut Manager, _: ()) -> Response<VoidMsg> {
                self.saved = Duration::default();
                self.start = None;
                *mgr |= self.display.set_str("0.000");
                Response::None
            }
            fn start(&mut self, mgr: &mut Manager, _: ()) -> Response<VoidMsg> {
                if let Some(start) = self.start {
                    self.saved += Instant::now() - start;
                    self.start = None;
                } else {
                    self.start = Some(Instant::now());
                    mgr.update_on_timer(Duration::new(0, 0), self.id(), 0);
                }
                Response::None
            }
        }
        impl kas::WidgetConfig {
            fn configure(&mut self, mgr: &mut Manager) {
                mgr.enable_alt_bypass(true);
            }
        }
        impl Handler {
            type Msg = VoidMsg;
            fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<VoidMsg> {
                match event {
                    Event::TimerUpdate(0) => {
                        if let Some(start) = self.start {
                            let dur = self.saved + (Instant::now() - start);
                            let text = format!("{}.{:03}", dur.as_secs(), dur.subsec_millis());
                            *mgr |= self.display.set_string(text);
                            mgr.update_on_timer(Duration::new(0, 1), self.id(), 0);
                        }
                        Response::None
                    }
                    _ => Response::Unhandled,
                }
            }
        }
    };

    let mut window = Window::new("Stopwatch", stopwatch);
    window.set_restrict_dimensions(true, true);
    Box::new(window)
}

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let theme = kas::theme::ShadedTheme::new()
        .with_colours("dark")
        .with_font_size(18.0);
    kas::shell::Toolkit::new(theme)?
        .with_boxed(make_window())?
        .run()
}
