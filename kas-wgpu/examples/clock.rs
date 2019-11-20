// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example (simple periodically updated display)
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;
use std::time::Duration;

use kas::class::HasText;
use kas::event::Callback;
use kas::event::EmptyMsg;
use kas::macros::make_widget;
use kas::widget::{Label, Window};
use kas::TkWindow;

fn main() {
    let mut window = Window::new(make_widget! {
        container(vertical) => EmptyMsg;
        struct {
            #[widget] date: Label = Label::new(""),
            #[widget] time: Label = Label::new("")
        }
        impl {
            fn on_tick(&mut self, tk: &mut dyn TkWindow) {
                let now = Local::now();
                self.date.set_text(tk, now.format("%Y %m %d").to_string());
                self.time.set_text(tk, now.format("%H:%M:%S").to_string());
                tk.redraw(self);
            }
        }
    });

    window.add_callback(Callback::Repeat(Duration::from_secs(1)), &|w, tk| {
        w.on_tick(tk)
    });

    let mut toolkit = kas_wgpu::Toolkit::new();
    toolkit.add(window).unwrap();
    toolkit.run()
}
