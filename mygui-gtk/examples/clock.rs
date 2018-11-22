//! Clock example (simple periodically updated display)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

// Could use chrono to print wall time, but don't want an extra dependency here
use std::time::Instant;

use mygui::display::Text;
use mygui::event::NoResponse;
use mygui::macros::make_widget;
use mygui::{SimpleWindow, Toolkit, TkWidget, Window, CallbackCond};

fn main() -> Result<(), mygui_gtk::Error> {
    let mut window = SimpleWindow::new(make_widget!(vertical => NoResponse;
            #[widget] display: Text = Text::from("0"),
            start_time: Instant = Instant::now();
            fn on_tick(&mut self, tk: &TkWidget) {
                let duration = Instant::now() - self.start_time;
                let secs = format!("{}", duration.as_secs());
                self.display.set_text(tk, &secs);
            }
        ));
    
    window.add_callback(CallbackCond::TimeoutMs(1000),
            |window, tk| window.get_mut().on_tick(tk) );
    
    let mut toolkit = mygui_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
