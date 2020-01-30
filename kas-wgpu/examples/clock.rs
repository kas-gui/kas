// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;
use log::info;
use std::time::Duration;

use kas::class::HasText;
use kas::event::Manager;
use kas::widget::{Label, Window};
use kas::{Widget, WidgetCore};

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let window = Window::new("Clock", {
        #[layout(vertical)]
        #[handler]
        #[derive(Clone, Debug, kas :: macros :: Widget)]
        struct Clock {
            #[core]
            core: kas::CoreData,
            #[layout_data]
            layout_data: <Self as kas::LayoutData>::Data,
            #[widget]
            date: Label,
            #[widget]
            time: Label,
        }
        impl Widget for Clock {
            fn configure(&mut self, mgr: &mut Manager) {
                mgr.update_on_timer(Duration::new(0, 0), self.id());
            }

            fn update_timer(&mut self, mgr: &mut Manager) -> Option<Duration> {
                let now = Local::now();
                self.date.set_text(mgr, now.format("%Y-%m-%d").to_string());
                self.time.set_text(mgr, now.format("%H:%M:%S").to_string());
                let ns = 1_000_000_000 - (now.time().nanosecond() % 1_000_000_000);
                info!("Requesting update in {}ns", ns);
                Some(Duration::new(0, ns))
            }
        }
        Clock {
            core: Default::default(),
            layout_data: Default::default(),
            date: Label::new(""),
            time: Label::new(""),
        }
    });

    let mut theme = kas_wgpu::SampleTheme::new();
    theme.set_font_size(32.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
