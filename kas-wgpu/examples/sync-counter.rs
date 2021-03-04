// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A counter synchronised between multiple windows

use kas::event::{Manager, Response, VoidMsg};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::view::{SharedRc, SingleView};
use kas::widget::{TextButton, Window};

#[derive(Clone, Debug, VoidMsg)]
enum Message {
    Decr,
    Incr,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[layout(row)]
        #[handler(msg = Message)]
        #[derive(Clone)]
        struct {
            #[widget] _ = TextButton::new_msg("−", Message::Decr),
            #[widget] _ = TextButton::new_msg("+", Message::Incr),
        }
    };

    let window = Window::new(
        "Counter",
        make_widget! {
            #[layout(column)]
            #[derive(Clone)]
            #[handler(msg = VoidMsg)]
            struct {
                // SingleView embeds a shared value, here default-constructed to 0
                #[widget(halign=centre)] counter: SingleView<SharedRc<i32>> = Default::default(),
                #[widget(handler = handle_button)] buttons -> Message = buttons,
            }
            impl {
                fn handle_button(&mut self, mgr: &mut Manager, msg: Message)
                    -> Response<VoidMsg>
                {
                    self.counter.update_value(mgr, |v| v + match msg {
                        Message::Decr => -1,
                        Message::Incr => 1,
                    });
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new().with_font_size(24.0);
    kas_wgpu::Toolkit::new(theme)?
        .with(window.clone())?
        .with(window)?
        .run()
}
