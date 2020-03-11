// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::event::{Manager, Response, UpdateHandle, VoidMsg, VoidResponse};
use kas::macros::{make_widget, VoidMsg};
use kas::widget::*;
use kas::{Horizontal, WidgetId};

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    Button,
    Check(bool),
    Radio(WidgetId),
    Edit(String),
    Slider(i32),
    Scroll(u32),
    Popup,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let radio = UpdateHandle::new();
    let widgets = make_widget! {
        #[widget]
        #[layout(grid)]
        #[handler(msg = Item)]
        struct {
            #[widget(row=0, col=0)] _ = Label::from("Label"),
            #[widget(row=0, col=1)] _ = Label::from("Hello world"),
            #[widget(row=1, col=0)] _ = Label::from("EditBox"),
            #[widget(row=1, col=1)] _ = EditBox::new("edit me")
                .on_activate(|text| Some(Item::Edit(text.to_string()))),
            #[widget(row=2, col=0)] _ = Label::from("TextButton"),
            #[widget(row=2, col=1)] _ = TextButton::new("Press me", Item::Button),
            #[widget(row=3, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("Check me").state(true)
                .on_toggle(|check| Item::Check(check)),
            #[widget(row=4, col=0)] _ = Label::from("RadioBox"),
            #[widget(row=4, col=1)] _ = RadioBox::new(radio, "radio box 1").state(false)
                .on_activate(|id| Item::Radio(id)),
            #[widget(row=5, col=0)] _ = Label::from("RadioBox"),
            #[widget(row=5, col=1)] _ = RadioBox::new(radio, "radio box 2").state(true)
                .on_activate(|id| Item::Radio(id)),
            #[widget(row=6, col=0)] _ = Label::from("Slider"),
            #[widget(row=6, col=1, handler = handle_slider)] _ =
                Slider::<i32, Horizontal>::new(-2, 2).with_value(0),
            #[widget(row=7, col=0)] _ = Label::from("ScrollBar"),
            #[widget(row=7, col=1, handler = handle_scroll)] _ =
                ScrollBar::<Horizontal>::new().with_limits(5, 2),
            #[widget(row=8)] _ = Label::from("Child window"),
            #[widget(row=8, col = 1)] _ = TextButton::new("Open", Item::Popup),
        }
        impl {
            fn handle_slider(&mut self, _: &mut Manager, msg: i32) -> Response<Item> {
                Response::Msg(Item::Slider(msg))
            }
            fn handle_scroll(&mut self, _: &mut Manager, msg: u32) -> Response<Item> {
                Response::Msg(Item::Scroll(msg))
            }
        }
    };

    let window = Window::new(
        "Widget Gallery",
        make_widget! {
            #[widget]
            #[layout(vertical)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] _ = make_widget! {
                    #[widget]
                    #[layout(vertical, frame)]
                    #[handler(msg = VoidMsg)]
                    struct {
                        #[widget(halign=centre)] _ = Label::from("Widget Gallery"),
                        #[widget(handler=set_theme)] _ = make_widget! {
                            #[widget]
                            #[layout(horizontal)]
                            #[handler(msg = &'static str)]
                            struct {
                                #[widget] _ = TextButton::new("Flat", "flat"),
                                #[widget] _ = TextButton::new("Shaded", "shaded"),
                            }
                        },
                        #[widget(handler=set_colour)] _ = make_widget! {
                            #[widget]
                            #[layout(horizontal)]
                            #[handler(msg = &'static str)]
                            struct {
                                #[widget] _ = TextButton::new("Default", "default"),
                                #[widget] _ = TextButton::new("Light", "light"),
                                #[widget] _ = TextButton::new("Dark", "dark"),
                            }
                        },
                    }
                    impl {
                        fn set_theme(&mut self, mgr: &mut Manager, name: &'static str)
                            -> VoidResponse
                        {
                            println!("Theme: {:?}", name);
                            #[cfg(not(feature = "stack_dst"))]
                            println!("Warning: switching themes requires feature 'stack_dst'");

                            mgr.adjust_theme(|theme| theme.set_theme(name));
                            VoidResponse::None
                        }
                        fn set_colour(&mut self, mgr: &mut Manager, name: &'static str)
                            -> VoidResponse
                        {
                            println!("Colour scheme: {:?}", name);
                            mgr.adjust_theme(|theme| theme.set_colours(name));
                            VoidResponse::None
                        }
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
                        Item::Radio(id) => println!("Radiobox: {}", id),
                        Item::Edit(s) => println!("Edited: {}", s),
                        Item::Slider(p) => println!("Slider: {}", p),
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

    #[cfg(feature = "stack_dst")]
    let theme = kas_theme::MultiTheme::builder()
        .add("shaded", kas_theme::ShadedTheme::new())
        .add("flat", kas_theme::FlatTheme::new())
        .build();
    #[cfg(not(feature = "stack_dst"))]
    let theme = kas_theme::ShadedTheme::new();

    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
