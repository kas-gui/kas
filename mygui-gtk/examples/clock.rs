//! Clock example (simple periodically updated display)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

extern crate chrono;

use chrono::prelude::*;

use mygui::display::Text;
use mygui::event::NoResponse;
use mygui::macros::make_widget;
use mygui::{SimpleWindow, Toolkit, TkWidget, CallbackCond};

fn main() -> Result<(), mygui_gtk::Error> {
    let mut window = SimpleWindow::new(make_widget!(vertical => NoResponse;
            #[widget] date: Text = Text::from(""),
            #[widget] time: Text = Text::from("");
            fn on_tick(&mut self, tk: &TkWidget) {
                let now = Local::now();
                self.date.set_text(tk, &now.format("%Y %m %d").to_string());
                self.time.set_text(tk, &now.format("%H:%M:%S").to_string());
            }
        ));
    
    window.add_callback(CallbackCond::TimeoutMs(1000), |w, tk| w.on_tick(tk) );
    
    let mut toolkit = mygui_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
