// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::event::{Manager, Response, VoidMsg, VoidResponse};
use kas::layout::Horizontal;
use kas::macros::{make_widget, VoidMsg};
use kas::widget::*;

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    Button,
    Check(bool),
    Edit(String),
    Scroll(u32),
    Popup,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let widgets = make_widget! {
        grid => Item;
        struct {
            #[widget(row=0, col=0)] _ = Label::from("Label"),
            #[widget(row=0, col=1)] _ = Label::from("Hello world"),
            #[widget(row=1, col=0)] _ = Label::from("EditBox"),
            #[widget(row=1, col=1)] _ = EditBox::new("edit me")
                .on_activate(|entry| Item::Edit(entry.to_string())),
            #[widget(row=2, col=0)] _ = Label::from("TextButton"),
            #[widget(row=2, col=1)] _ = TextButton::new("Press me", Item::Button),
            #[widget(row=3, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("Check me")
                .on_toggle(|check| Item::Check(check)),
            #[widget(row=4, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=4, col=1)] _ = CheckBox::new("").state(true)
                .on_toggle(|check| Item::Check(check)),
            #[widget(row=5, col=0)] _ = Label::from("ScrollBar"),
            #[widget(row=5, col=1, handler = handle_scroll)] _ =
                ScrollBar::<Horizontal>::new().with_limits(5, 2),
            #[widget(row=8)] _ = Label::from("Child window"),
            #[widget(row=8, col = 1)] _ = TextButton::new("Open", Item::Popup),
        }
        impl {
            fn handle_scroll(&mut self, _: &mut Manager, msg: u32) -> Response<Item> {
                Response::Msg(Item::Scroll(msg))
            }
        }
    };

    let window = Window::new(
        "Widget Gallery",
        make_widget! {
            vertical => VoidMsg;
            struct {
                #[widget] _ = make_widget! {
                    frame => VoidMsg;
                    struct {
                        #[widget] _ = Label::from("Widget Gallery"),
                    }
                },
                #[widget(handler = activations)] _ = ScrollRegion::new(widgets).with_auto_bars(true),
            }
            impl {
                fn activations(&mut self, mgr: &mut Manager, item: Item)
                    -> VoidResponse
                {
                    match item {
                        Item::Button => println!("Clicked!"),
                        Item::Check(b) => println!("Checkbox: {}", b),
                        Item::Edit(s) => println!("Edited: {}", s),
                        Item::Scroll(p) => println!("ScrollBar: {}", p),
                        Item::Popup => {
                            let window = MessageBox::new("Popup", "Hello!");
                            mgr.add_window(Box::new(window));
                        }
                    };
                    VoidResponse::None
                }
            }
        },
    );

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
