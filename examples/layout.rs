// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Demonstration of widget and text layouts

use kas::Window;
use kas::layout::AlignHints;
use kas::widgets::{AdaptWidget, CheckBox, EditBox, ScrollLabel, grid};

const LIPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Morbi nunc mi, consequat eget urna ut, auctor luctus mi. Sed molestie mi est. Sed non ligula ante. Curabitur ac molestie ante, nec sodales eros. In non arcu at turpis euismod bibendum ut tincidunt eros. Suspendisse blandit maximus nisi, viverra hendrerit elit efficitur et. Morbi ut facilisis eros. Vivamus dignissim, sapien sed mattis consectetur, libero leo imperdiet turpis, ac pulvinar libero purus eu lorem. Etiam quis sollicitudin urna. Integer vitae erat vel neque gravida blandit ac non quam.";
const CRASIT: &str = "Cras sit amet justo ipsum. Aliquam in nunc posuere leo egestas laoreet convallis eu libero. Nullam ut massa ante. Cras vitae velit pharetra, euismod nisl suscipit, feugiat nulla. Aenean consectetur, diam non tristique iaculis, nisl lectus hendrerit sapien, nec rhoncus mi sem non odio. Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Nulla a lorem eu ipsum faucibus placerat ac quis quam. Curabitur justo ligula, laoreet nec ultrices eu, scelerisque non metus. Mauris sit amet est enim. Mauris risus eros, accumsan ut iaculis sit amet, sagittis facilisis neque. Nunc venenatis risus nec purus malesuada, a tristique arcu efficitur. Nulla suscipit arcu nibh. Cras facilisis nibh a gravida aliquet. Praesent fringilla felis a tristique luctus.";

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let ui = grid! {
        (1, 0) => "Layout demo",
        (2, 0) => CheckBox::new(|_, _| true),
        (0..3, 1) => ScrollLabel::new(LIPSUM),
        (0, 2) => "abc אבג def".align(AlignHints::CENTER),
        (1..3, 3) => ScrollLabel::new(CRASIT).align(AlignHints::STRETCH),
        (0, 3) => EditBox::text("A small\nsample\nof text").with_multi_line(true),
    };
    let window = Window::new(ui, "Layout demo");

    kas::runner::Runner::new(())?.with(window).run()
}
