// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::prelude::*;
use kas::widget::view::{Accessor, ListView};
use kas::widget::{EditBox, ScrollBars, Window};

#[cfg(not(feature = "generator"))]
mod data {
    use kas::widget::view::{FilterAccessor, SharedConst};
    use std::{cell::RefCell, rc::Rc};

    type SC = &'static SharedConst<[&'static str]>;
    pub type Shared = Rc<RefCell<FilterAccessor<usize, SC>>>;

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

    pub fn get() -> Shared {
        Rc::new(RefCell::new(FilterAccessor::new_visible(MONTHS.into())))
    }
}

// Implementation which generates dates, allowing testing with large numbers of entries
// Since entries are generated on demand there is no penalty to having very large
// numbers, *except* that this filter is O(n) in both memory usage and update time.
#[cfg(feature = "generator")]
mod data {
    use chrono::{DateTime, Duration, Local};
    use kas::widget::view::{Accessor, FilterAccessor};
    use std::{cell::RefCell, rc::Rc};

    // pub type Shared = Rc<RefCell<DateGenerator>>;
    pub type Shared = Rc<RefCell<FilterAccessor<usize, DateGenerator>>>;

    #[derive(Debug)]
    pub struct DateGenerator {
        start: DateTime<Local>,
        end: DateTime<Local>,
        step: Duration,
    }

    impl Accessor<usize> for DateGenerator {
        type Item = String;
        fn len(&self) -> usize {
            let dur = self.end - self.start;
            let secs = dur.num_seconds();
            let step_secs = self.step.num_seconds();
            1 + ((secs - 1) / step_secs) as usize
        }

        fn get(&self, index: usize) -> Self::Item {
            let date = self.start + self.step * index as i32;
            date.format("%A %e %B %Y, %T").to_string()
        }
    }

    pub fn get() -> Shared {
        Rc::new(RefCell::new(FilterAccessor::new_visible(DateGenerator {
            start: Local::now(),
            end: Local::now() + Duration::days(365),
            step: Duration::seconds(999),
        })))
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let data = data::get();
    println!("filter-list: {} entries", data.borrow().len());
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
                #[widget] list = ScrollBars::new(ListView::<kas::Down, data::Shared>::new(data)),
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
