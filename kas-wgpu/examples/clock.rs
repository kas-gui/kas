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
use kas::event::{Callback, Manager, VoidMsg};
use kas::macros::make_widget;
use kas::widget::{Label, Window};

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let mut window = Window::new(
        "Clock",
        make_widget! {
            #[widget]
            #[layout(vertical)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] date: Label = Label::new(""),
                #[widget] time: Label = Label::new("")
            }
            impl {
                fn on_tick(&mut self, mgr: &mut Manager) {
                    let now = Local::now();
                    self.date.set_text(mgr, now.format("%Y-%m-%d").to_string());
                    self.time.set_text(mgr, now.format("%H:%M:%S").to_string());
                }
            }
        },
    );

    window.add_callback(Callback::Repeat(Duration::from_secs(1)), &|w, mgr| {
        w.on_tick(mgr)
    });

    let mut theme = kas_wgpu::SampleTheme::new();
    theme.set_font_size(32.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
