// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows
#![feature(proc_macro_hygiene)]

use std::cell::RefCell;

use kas::class::HasText;
use kas::event::{Manager, UpdateHandle, VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::{Label, TextButton, Window};
use kas::{Widget, WidgetCore};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

thread_local! {
    // Save ourselves usage of thread-safe primitives by keeping to a single thread.
    static COUNTER: RefCell<i32> = RefCell::new(0);
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[widget]
        #[layout(horizontal)]
        #[handler(msg = Message)]
        struct {
            #[widget] _ = TextButton::new("−", Message::Decr),
            #[widget] _ = TextButton::new("+", Message::Incr),
        }
    };

    let handle = UpdateHandle::new();

    let window = Window::new(
        "Counter",
        make_widget! {
            #[layout(vertical)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] display: Label = Label::from("0"),
                #[widget(handler = handle_button)] buttons -> Message = buttons,
                handle: UpdateHandle = handle,
            }
            impl Widget {
                fn configure(&mut self, mgr: &mut Manager) {
                    mgr.update_on_handle(self.handle, self.id());
                }

                fn update_handle(&mut self, mgr: &mut Manager, _: UpdateHandle) {
                    let c = COUNTER.with(|c| *c.borrow());
                    self.display.set_text(mgr, c.to_string());
                }
            }
            impl {
                fn handle_button(&mut self, mgr: &mut Manager, msg: Message)
                    -> VoidResponse
                {
                    COUNTER.with(|c| {
                        let mut c = c.borrow_mut();
                        *c += match msg {
                            Message::Decr => -1,
                            Message::Incr => 1,
                        };
                    });
                    mgr.trigger_update(self.handle);
                    VoidResponse::None
                }
            }
        },
    );

    let mut theme = kas_wgpu::SampleTheme::new();
    theme.set_font_size(24.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window.clone())?;
    toolkit.add(window)?;
    toolkit.run()
}
