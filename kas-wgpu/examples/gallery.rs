// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::event::EmptyMsg;
use kas::macros::{make_widget, EmptyMsg};
use kas::widget::*;
use kas::TkWindow;

#[derive(Clone, Debug, EmptyMsg)]
enum Item {
    None,
    Button,
    Check(bool),
    Edit(String),
}

fn main() -> Result<(), winit::error::OsError> {
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
            #[widget(row=5)] _ = Label::from("A"),
            #[widget(row=6)] _ = Label::from("few"),
            #[widget(row=7)] _ = Label::from("more"),
            #[widget(row=8)] _ = Label::from("rows"),
            #[widget(row=8, col = 1)] _ = TextButton::new("Another button", Item::Button),
        }
    };

    let window = Window::new(make_widget! {
        vertical => EmptyMsg;
        struct {
            #[widget] _ = make_widget! {
                frame => EmptyMsg;
                struct {
                    #[widget] _ = Label::from("Widget Gallery"),
                }
            },
            #[widget(handler = activations)] _ = ScrollRegion::new(widgets),
        }
        impl {
            fn activations(&mut self, _: &mut dyn TkWindow, item: Item)
                -> EmptyMsg
            {
                match item {
                    Item::None => (),
                    Item::Button => println!("Clicked!"),
                    Item::Check(b) => println!("Checkbox: {}", b),
                    Item::Edit(s) => println!("Edited: {}", s),
                };
                EmptyMsg
            }
        }
    });

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme);
    toolkit.add(window)?;
    toolkit.run()
}
