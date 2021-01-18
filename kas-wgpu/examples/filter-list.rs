// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::event::UpdateHandle;
use kas::prelude::*;
use kas::widget::view::{Accessor, ListView};
use kas::widget::{EditBox, Window};
use std::cell::RefCell;
use std::rc::Rc;

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
    update: UpdateHandle,
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
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let data = Rc::new(RefCell::new(FilterAccessor {
        data: MONTHS,
        filter: "".to_string(),
        update: UpdateHandle::new(),
    }));
    let data2 = data.clone();
    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[layout(down)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] filter = EditBox::new("").on_edit(move |text, mgr| {
                    let mut data = data2.borrow_mut();
                    data.filter.clear();
                    data.filter.push_str(text);
                    mgr.trigger_update(data.update, 0);
                    None
                }),
                #[widget] list = ListView::<kas::Down, Rc<RefCell<FilterAccessor>>>::new(data),
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
