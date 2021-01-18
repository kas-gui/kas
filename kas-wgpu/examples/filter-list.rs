// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::prelude::*;
use kas::widget::view::{FilterAccessor, ListView, SharedConst};
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

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    type SC = &'static SharedConst<[&'static str]>;
    type FA = Rc<RefCell<FilterAccessor<usize, SC>>>;
    let data: SC = MONTHS.into();
    let data = Rc::new(RefCell::new(FilterAccessor::new_visible(data)));
    let data2 = data.clone();
    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[layout(down)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] filter = EditBox::new("").on_edit(move |text, mgr| {
                    // Note: this method of caseless matching is not unicode compliant!
                    // https://stackoverflow.com/questions/47298336/case-insensitive-string-matching-in-rust
                    let text = text.to_uppercase();
                    let update = data2
                        .borrow_mut()
                        .update_filter(|s| s.to_uppercase().contains(&text));
                    mgr.trigger_update(update, 0);
                    None
                }),
                #[widget] list = ListView::<kas::Down, FA>::new(data),
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
