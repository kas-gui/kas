// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widget example
#![feature(proc_macro_hygiene)]

use kas::class::HasText;
use kas::event::{Callback, Response, VoidMsg};
use kas::layout::Vertical;
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{DynVec, EditBox, Label, ScrollRegion, TextButton, Window};
use kas::TkWindow;

#[derive(Clone, Debug, VoidMsg)]
enum Control {
    Decr,
    Incr,
    Set,
}

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Set(usize),
}

fn main() -> Result<(), kas_wgpu::Error> {
    let controls = make_widget! {
        horizontal => Message;
        struct {
            #[widget] _ = Label::new("Number of rows:"),
            #[widget(handler = handler)] edit: impl HasText = EditBox::new("3").on_activate(|_| Control::Set),
            #[widget(handler = handler)] _ = TextButton::new("Set", Control::Set),
            #[widget(handler = handler)] _ = TextButton::new("−", Control::Decr),
            #[widget(handler = handler)] _ = TextButton::new("+", Control::Incr),
        }
        impl {
            fn handler(&mut self, tk: &mut dyn TkWindow, msg: Control) -> Response<Message> {
                match self.edit.get_text().parse::<usize>() {
                    Ok(mut n) => {
                        match msg {
                            Control::Decr => {
                                n = n.saturating_sub(1);
                                self.edit.set_string(tk, n.to_string());
                            },
                            Control::Incr => {
                                n = n.saturating_add(1);
                                self.edit.set_string(tk, n.to_string());
                            },
                            Control::Set => ()
                        }
                        Message::Set(n).into()
                    }
                    _ => {
                        self.edit.set_string(tk, "0".to_string());
                        Message::Set(0).into()
                    }
                }
            }
        }
    };
    let mut window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            vertical => VoidMsg;
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget(handler = handler)] controls -> Message = controls,
                #[widget] list: ScrollRegion<DynVec<Vertical, EditBox<()>>> = ScrollRegion::new(DynVec::new(Vertical, vec![])),
            }
            impl {
                fn handler(&mut self, tk: &mut dyn TkWindow, msg: Message) -> Response<VoidMsg>
                {
                    match msg {
                        Message::Set(n) => {
                            self.list.inner_mut().resize_with(tk, n, |i| EditBox::new(i.to_string()));
                        }
                    };
                    Response::None
                }
            }
        },
    );

    window.add_callback(Callback::Start, &|w, tk| {
        let _ = w.handler(tk, Message::Set(3));
    });

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
