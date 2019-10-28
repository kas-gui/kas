// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![feature(proc_macro_hygiene)]

use kas::control::{CheckBox, TextButton};
use kas::macros::make_widget;
use kas::text::{Entry, Label};
use kas::SimpleWindow;

fn main() -> Result<(), winit::error::OsError> {
    let lipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi nunc mi, consequat eget urna ut, auctor luctus mi. Sed molestie mi est. Sed non ligula ante. Curabitur ac molestie ante, nec sodales eros. In non arcu at turpis euismod bibendum ut tincidunt eros. Suspendisse blandit maximus nisi, viverra hendrerit elit efficitur et. Morbi ut facilisis eros. Vivamus dignissim, sapien sed mattis consectetur, libero leo imperdiet turpis, ac pulvinar libero purus eu lorem. Etiam quis sollicitudin urna. Integer vitae erat vel neque gravida blandit ac non quam.";
    let widgets = make_widget! {
        container(grid) => ();
        struct {
            #[widget(row=0, col=0)] _ = Label::from("Label"),
            #[widget(row=0, col=1)] _ = Label::from("Hello world"),
            #[widget(row=1, col=0)] _ = Label::from("Entry"),
            #[widget(row=1, col=1)] _ = Entry::new("edit me"),
            #[widget(row=2, col=0)] _ = Label::from("TextButton"),
            #[widget(row=2, col=1)] _ = TextButton::new_on("Press me", || println!("Clicked!")),
            #[widget(row=3, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=3, col=1)] _ = CheckBox::new("Check me"),
            #[widget(row=4, col=0)] _ = Label::from("CheckBox"),
            #[widget(row=4, col=1)] _ = CheckBox::new("").state(true),
            #[widget(row=5, col=0, cspan=2)] _ = Label::from(lipsum),
        }
    };

    let window = SimpleWindow::new(make_widget! {
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
