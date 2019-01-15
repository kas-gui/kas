// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use std::{cell::RefCell, rc::Rc};
use std::fmt::Write;
use std::time::{Duration, Instant};

use kas::control::TextButton;
use kas::display::Text;
use kas::event::{NoResponse};
use kas::macros::{NoResponse, make_widget};
use kas::{Class, SimpleWindow, Toolkit, TkWidget, CallbackCond, Window};

#[derive(Debug, NoResponse)]
enum Control {
    None,
    Reset,
    Start,
}

// Unlike most examples, we encapsulate the GUI configuration into a function.
// There's no reason for this, but it demonstrates usage of Toolkit::add_rc
fn make_window() -> Rc<RefCell<Window>> {
    trait SetText {
        fn set_text(&mut self, tk: &TkWidget, text: &str);
    }
    
    let stopwatch = make_widget! {
        horizontal => NoResponse;
        struct {
            #[widget] display: impl SetText = make_widget!{
                single => NoResponse;
                class = Class::Frame;
                struct {
                    #[widget] display: Text = Text::from("0.000"),
                }
                impl SetText {
                    fn set_text(&mut self, tk: &TkWidget, text: &str) {
                        self.display.set_text(tk, text);
                    }
                }
            },
            #[widget(handler = handle_button)] b_reset = TextButton::new("⏮", || Control::Reset),
            #[widget(handler = handle_button)] b_start = TextButton::new("⏯", || Control::Start),
            saved: Duration = Duration::default(),
            start: Option<Instant> = None,
            dur_buf: String = String::default(),
        }
        impl {
            fn handle_button(&mut self, tk: &TkWidget, msg: Control) -> NoResponse {
                match msg {
                    Control::None => {}
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
                NoResponse
            }
            
            fn on_tick(&mut self, tk: &TkWidget) {
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
    
    let mut window = SimpleWindow::new(stopwatch);
    
    window.add_callback(CallbackCond::TimeoutMs(16), &|w, tk| w.on_tick(tk) );
    
    Rc::new(RefCell::new(window))
}

fn main() -> Result<(), kas_gtk::Error> {
    let mut toolkit = kas_gtk::Toolkit::new()?;
    toolkit.add_rc(make_window());
    toolkit.main();
    Ok(())
}
