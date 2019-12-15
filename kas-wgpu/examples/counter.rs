// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Counter example (simple button)
#![feature(proc_macro_hygiene)]

use kas::class::HasText;
use kas::event::{VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Label, TextButton, Window};
use kas::TkWindow;

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), winit::error::OsError> {
    let buttons = make_widget! {
        horizontal => Message;
        struct {
            #[widget] _ = TextButton::new("−", Message::Decr),
            #[widget] _ = TextButton::new("+", Message::Incr),
        }
    };
    let window = Window::new(make_widget! {
        vertical => VoidMsg;
        struct {
            #[widget] display: Label = Label::from("0"),
            #[widget(handler = handle_button)] buttons -> Message = buttons,
            counter: usize = 0,
        }
        impl {
            fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: Message)
                -> VoidResponse
            {
                match msg {
                    Message::Decr => {
                        self.counter = self.counter.saturating_sub(1);
                        self.display.set_text(tk, self.counter.to_string());
                    }
                    Message::Incr => {
                        self.counter = self.counter.saturating_add(1);
                        self.display.set_text(tk, self.counter.to_string());
                    }
                };
                VoidResponse::None
            }
        }
    });

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme);
    toolkit.add(window)?;
    toolkit.run()
}
