// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::prelude::*;
use kas::widget::view::{Accessor, ListView};
use kas::widget::{EditBox, Window};

const MONTHS: &[&str] = &[
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// Naive implementation not intended for big data
#[derive(Debug)]
struct FilterAccessor {
    data: &'static [&'static str],
    filter: String,
}
impl Accessor<usize> for FilterAccessor {
    type Item = &'static str;
    fn len(&self) -> usize {
        self.data
            .iter()
            .filter(|d| d.contains(&self.filter))
            .count()
    }
    fn get(&self, index: usize) -> &'static str {
        self.data
            .iter()
            .filter(|d| d.contains(&self.filter))
            .nth(index)
            .unwrap()
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let data = FilterAccessor {
        data: MONTHS,
        filter: "".to_string(),
    };
    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[layout(down)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(handler=update_filter)] filter = EditBox::new("").on_edit(|text| Some(text.to_string())),
                #[widget] list: ListView::<kas::Down, FilterAccessor> = ListView::new(data),
            }
            impl {
                fn update_filter(&mut self, mgr: &mut Manager, text: String) -> Response<VoidMsg> {
                    self.list.data_mut().filter = text;
                    self.list.update_view(mgr);
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
