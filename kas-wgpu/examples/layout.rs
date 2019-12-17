// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Gallery of all widgets
#![recursion_limit = "256"]

use kas::event::VoidMsg;
use kas::make_widget;
use kas::widget::{CheckBox, EditBox, Label, Window};

fn main() -> Result<(), winit::error::OsError> {
    let lipsum = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi nunc mi, consequat eget urna ut, auctor luctus mi. Sed molestie mi est. Sed non ligula ante. Curabitur ac molestie ante, nec sodales eros. In non arcu at turpis euismod bibendum ut tincidunt eros. Suspendisse blandit maximus nisi, viverra hendrerit elit efficitur et. Morbi ut facilisis eros. Vivamus dignissim, sapien sed mattis consectetur, libero leo imperdiet turpis, ac pulvinar libero purus eu lorem. Etiam quis sollicitudin urna. Integer vitae erat vel neque gravida blandit ac non quam.";
    let crasit = "Cras sit amet justo ipsum. Aliquam in nunc posuere leo egestas laoreet convallis eu libero. Nullam ut massa ante. Cras vitae velit pharetra, euismod nisl suscipit, feugiat nulla. Aenean consectetur, diam non tristique iaculis, nisl lectus hendrerit sapien, nec rhoncus mi sem non odio. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Nulla a lorem eu ipsum faucibus placerat ac quis quam. Curabitur justo ligula, laoreet nec ultrices eu, scelerisque non metus. Mauris sit amet est enim. Mauris risus eros, accumsan ut iaculis sit amet, sagittis facilisis neque. Nunc venenatis risus nec purus malesuada, a tristique arcu efficitur. Nulla suscipit arcu nibh. Cras facilisis nibh a gravida aliquet. Praesent fringilla felis a tristique luctus.";

    let window = Window::new(
        "Layout demo",
        make_widget! {
            grid => VoidMsg;
            struct {
                #[widget(row=0, col=2)] _ = Label::from("Layout demo"),
                #[widget(row=1, col=1, cspan=3)] _ = Label::from(lipsum),
                #[widget(row=1, col=4)] _ = CheckBox::new(""),
                #[widget(row=2, col=0)] _ = Label::from("Text"),
                #[widget(row=2, col=2, cspan=2, rspan=2)] _ = Label::from(crasit),
                #[widget(row=3, col=1)] _ = EditBox::new("edit"),
                #[widget(row=0, col=3)] _ = Label::from("<->"),
            }
        },
    );

    let theme = kas_wgpu::SampleTheme::new();
    let mut toolkit = kas_wgpu::Toolkit::new(theme);
    toolkit.add(window)?;
    toolkit.run()
}
