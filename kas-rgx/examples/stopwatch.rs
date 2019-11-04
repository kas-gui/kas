// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)
#![feature(proc_macro_hygiene)]

use std::fmt::Write;
use std::time::{Duration, Instant};

use kas::class::HasText;
use kas::event::{Condition, Response};
use kas::macros::make_widget;
use kas::widget::{Label, TextButton, Window};
use kas::TkWidget;

#[derive(Debug)]
enum Control {
    Reset,
    Start,
}

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_boxed
fn make_window() -> Box<dyn kas::Window> {
    let stopwatch = make_widget! {
        container(horizontal) => ();
        struct {
            #[widget] display: impl HasText = make_widget!{
                frame => ();
                struct {
                    #[widget] display: Label = Label::from("0.000"),
                }
                impl HasText {
                    fn get_text(&self) -> &str {
                        self.display.get_text()
                    }
                    fn set_string(&mut self, tk: &mut dyn TkWidget, text: String) {
                        self.display.set_text(tk, text);
                    }
                }
            },
            #[widget(handler = handle_button)] b_reset = TextButton::new_on("reset", || Control::Reset),
            #[widget(handler = handle_button)] b_start = TextButton::new_on("start / stop", || Control::Start),
            saved: Duration = Duration::default(),
            start: Option<Instant> = None,
            dur_buf: String = String::default(),
        }
        impl {
            fn handle_button(&mut self, tk: &mut dyn TkWidget, msg: Control) -> Response<()> {
                match msg {
                    Control::Reset => {
                        self.saved = Duration::default();
                        self.start = None;
                        self.display.set_text(tk, "0.000");
                    }
                    Control::Start => {
                        if let Some(start) = self.start {
                            self.saved += Instant::now() - start;
                            self.start = None;
                        } else {
                            self.start = Some(Instant::now());
                        }
                    }
                }
                Response::None
            }

            fn on_tick(&mut self, tk: &mut dyn TkWidget) {
                if let Some(start) = self.start {
                    let dur = self.saved + (Instant::now() - start);
                    self.dur_buf.clear();
                    self.dur_buf.write_fmt(format_args!(
                        "{}.{:03}",
                        dur.as_secs(),
                        dur.subsec_millis()
                    )).unwrap();
                    self.display.set_text(tk, &self.dur_buf);
                }
            }
        }
    };

    let mut window = Window::new(stopwatch);

    window.add_callback(Condition::Repeat(Duration::from_millis(16)), &|w, tk| {
        w.on_tick(tk)
    });

    Box::new(window)
}

fn main() -> Result<(), winit::error::OsError> {
    let mut toolkit = kas_rgx::Toolkit::new();
    toolkit.add_boxed(make_window())?;
    toolkit.run()
}
