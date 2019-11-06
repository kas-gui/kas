// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::macros::make_widget;
use kas::widget::*;

fn main() -> Result<(), winit::error::OsError> {
    let widgets = make_widget! {
        container(grid) => ();
        struct {
            #[widget(row=0, col=0)] _ = Label::from("Label"),
            #[widget(row=0, col=1)] _ = Label::from("Hello world"),
            #[widget(row=1, col=0)] _ = Label::from("Entry"),
            #[widget(row=1, col=1)] _ = Entry::new("edit me"),
            #[widget(row=2, col=0)] _ = Label::from("TextButton"),
            #[widget(row=2, col=1)] _ = TextButton::new("Press me", println!("Clicked!")),
            #[widget(row=3, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("Check me"),
            #[widget(row=4, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=4, col=1)] _ = CheckBox::new("").state(true),
        }
    };

    let window = Window::new(make_widget! {
        container(vertical) => ();
        struct {
            #[widget] _ = Label::from("Widget Gallery"),
            #[widget] _ = make_widget! {
                frame => ();
                struct {
                    #[widget] _ = widgets
                }
            }
        }
    });

    let mut toolkit = kas_rgx::Toolkit::new();
    toolkit.add(window)?;
    toolkit.run()
}
