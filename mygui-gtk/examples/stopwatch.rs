//! Counter example (simple button)
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use std::fmt::Write;
use std::time::{Duration, Instant};

use mygui::control::TextButton;
use mygui::display::Text;
use mygui::event::{Handler, NoResponse};
use mygui::macros::{NoResponse, Widget, make_widget};
use mygui::{Class, CoreData, Widget, SimpleWindow, Toolkit, TkWidget, CallbackCond};

#[derive(Debug, NoResponse)]
enum Control {
    None,
    Reset,
    Start,
}

fn main() -> Result<(), mygui_gtk::Error> {
    trait SetText {
        fn set_text(&mut self, tk: &TkWidget, text: &str);
    }
    
    #[layout(horizontal)]
    #[widget(class = Class::Container)]
    #[handler(response = NoResponse, generics = <>
        where BR: Handler<Response = Control>, BS: Handler<Response = Control>,
        D: Handler<Response = NoResponse>)]
    #[derive(Clone, Debug, Widget)]
    struct Stopwatch<BR: Widget, BS: Widget, D: Widget + SetText> {
        #[core] core: CoreData,
        #[widget] display: D,
        #[widget(handler = handle_button)] b_reset: BR,
        #[widget(handler = handle_button)] b_start: BS,
        saved: Duration,
        start: Option<Instant>,
        dur_buf: String,
    }
    
    impl<BR: Widget, BS: Widget, D: Widget + SetText> Stopwatch<BR, BS, D> {
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
    
    let stopwatch = Stopwatch {
        core: CoreData::default(),
        display: make_widget!{
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
        b_reset: TextButton::new("⏮", || Control::Reset),
        b_start: TextButton::new("⏯", || Control::Start),
        saved: Duration::default(),
        start: None,
        dur_buf: String::default(),
    };
    
    let mut window = SimpleWindow::new(stopwatch);
    
    window.add_callback(CallbackCond::TimeoutMs(16), |w, tk| w.on_tick(tk) );
    
    let mut toolkit = mygui_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
