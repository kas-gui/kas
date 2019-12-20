// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Clock example, with the make_widget macro expanded
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;
use std::time::Duration;

use kas::class::HasText;
use kas::event::Callback;
use kas::widget::{Label, Window};
use kas::{TkWindow, WidgetCore};

fn main() -> Result<(), kas_wgpu::Error> {
    let mut window = Window::new("Clock", {
        #[widget (layout = vertical)]
        #[handler]
        #[derive(Clone, Debug, kas :: macros :: Widget)]
        struct AnonWidget {
            #[core]
            core: kas::CoreData,
            #[layout_data]
            layout_data: <Self as kas::LayoutData>::Data,
            #[widget]
            date: Label,
            #[widget]
            time: Label,
        }
        impl AnonWidget {
            fn on_tick(&mut self, tk: &mut dyn TkWindow) {
                let now = Local::now();
                self.date.set_text(tk, now.format("%Y %m %d").to_string());
                self.time.set_text(tk, now.format("%H:%M:%S").to_string());
                tk.redraw(self.id());
            }
        }
        AnonWidget {
            core: Default::default(),
            layout_data: Default::default(),
            date: Label::new(""),
            time: Label::new(""),
        }
    });

    window.add_callback(Callback::Repeat(Duration::from_secs(1)), &|w, tk| {
        w.on_tick(tk)
    });

    let mut theme = kas_wgpu::SampleTheme::new();
    theme.set_font_size(32.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
