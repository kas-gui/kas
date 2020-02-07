// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)
#![feature(proc_macro_hygiene)]

use std::fmt::Write;
use std::time::{Duration, Instant};

use kas::class::HasText;
use kas::event::{Manager, Response, VoidMsg};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Label, TextButton, Window};
use kas::{Widget, WidgetCore};

#[derive(Clone, Debug, VoidMsg)]
enum Control {
    Reset,
    Start,
}

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Window> {
    let stopwatch = make_widget! {
        #[layout(horizontal)]
        #[handler(msg = VoidMsg)]
        struct {
            #[widget] display: impl HasText = make_widget!{
                #[widget]
                #[layout(single, frame)]
                #[handler(msg = VoidMsg)]
                struct {
                    #[widget] display: Label = Label::from("0.000"),
                }
                impl HasText {
                    fn get_text(&self) -> &str {
                        self.display.get_text()
                    }
                    fn set_string(&mut self, mgr: &mut Manager, text: String) {
                        self.display.set_text(mgr, text);
                    }
                }
            },
            #[widget(handler = handle_button)] b_reset = TextButton::new("reset", Control::Reset),
            #[widget(handler = handle_button)] b_start = TextButton::new("start / stop", Control::Start),
            saved: Duration = Duration::default(),
            start: Option<Instant> = None,
            dur_buf: String = String::default(),
        }
        impl {
            fn handle_button(&mut self, mgr: &mut Manager, msg: Control) -> Response<VoidMsg> {
                match msg {
                    Control::Reset => {
                        self.saved = Duration::default();
                        self.start = None;
                        self.display.set_text(mgr, "0.000");
                    }
                    Control::Start => {
                        if let Some(start) = self.start {
                            self.saved += Instant::now() - start;
                            self.start = None;
                        } else {
                            self.start = Some(Instant::now());
                            mgr.update_on_timer(Duration::new(0, 0), self.id());
                        }
                    }
                }
                Response::None
            }
        }
        impl Widget {
            fn update_timer(&mut self, mgr: &mut Manager) -> Option<Duration> {
                if let Some(start) = self.start {
                    let dur = self.saved + (Instant::now() - start);
                    self.dur_buf.clear();
                    self.dur_buf.write_fmt(format_args!(
                        "{}.{:03}",
                        dur.as_secs(),
                        dur.subsec_millis()
                    )).unwrap();
                    self.display.set_text(mgr, &self.dur_buf);
                    Some(Duration::new(0, 1))
                } else {
                    None
                }
            }
        }
    };

    let mut window = Window::new("Stopwatch", stopwatch);
    window.set_enforce_size(true, true);
    Box::new(window)
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add_boxed(make_window())?;
    toolkit.run()
}
