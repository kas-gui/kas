// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example (simple periodically updated display)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;

use kas::callback::Condition;
use kas::display::Text;
use kas::event::NoResponse;
use kas::macros::make_widget;
use kas::HasText;
use kas::{SimpleWindow, Toolkit, TkWidget};

fn main() -> Result<(), kas_gtk::Error> {
    let mut window = SimpleWindow::new(make_widget! {
            container(vertical) => NoResponse;
            struct {
                #[widget] date: Text = Text::new(),
                #[widget] time: Text = Text::new()
            }
            impl {
                fn on_tick(&mut self, tk: &TkWidget) {
                    let now = Local::now();
                    self.date.set_text(tk, &now.format("%Y %m %d").to_string());
                    self.time.set_text(tk, &now.format("%H:%M:%S").to_string());
                }
            }
        });
    
    window.add_callback(&|w, tk| w.on_tick(tk),
            &[Condition::Start, Condition::TimeoutSec(1)]);
    
    let mut toolkit = kas_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
