// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::dir::Down;
use kas::event::UpdateHandle;
use kas::prelude::*;
use kas::widget::view::{ListData, ListMsg, ListView, SelectionMode, SimpleCaseInsensitiveFilter};
use kas::widget::{EditBox, Label, RadioBox, ScrollBars, Window};

#[cfg(not(feature = "generator"))]
mod data {
    use kas::widget::view::{FilteredList, SharedConst, SimpleCaseInsensitiveFilter};
    use std::rc::Rc;

    type SC = &'static SharedConst<[&'static str]>;
    pub type Shared = Rc<FilteredList<SC, SimpleCaseInsensitiveFilter>>;

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
        let filter = SimpleCaseInsensitiveFilter::new("");
        Rc::new(FilteredList::new(MONTHS.into(), filter))
    }
}

// Implementation which generates dates, allowing testing with large numbers of entries
// Since entries are generated on demand there is no penalty to having very large
// numbers, *except* that this filter is O(n) in both memory usage and update time.
#[cfg(feature = "generator")]
mod data {
    use chrono::{DateTime, Duration, Local};
    use kas::conv::Conv;
    use kas::widget::view::{FilteredList, ListData, SimpleCaseInsensitiveFilter};
    use std::rc::Rc;

    // Alternative: unfiltered version (must (de)comment a few bits of code)
    // pub type Shared = DateGenerator;
    pub type Shared = Rc<FilteredList<DateGenerator, SimpleCaseInsensitiveFilter>>;

    #[derive(Debug)]
    pub struct DateGenerator {
        start: DateTime<Local>,
        end: DateTime<Local>,
        step: Duration,
    }

    impl DateGenerator {
        fn gen(&self, index: usize) -> String {
            let date = self.start + self.step * i32::conv(index);
            date.format("%A %e %B %Y, %T").to_string()
        }
    }
    impl ListData for DateGenerator {
        type Key = usize;
        type Item = String;
        fn len(&self) -> usize {
            let dur = self.end - self.start;
            let secs = dur.num_seconds();
            let step_secs = self.step.num_seconds();
            1 + usize::conv((secs - 1) / step_secs)
        }

        fn get_cloned(&self, index: &usize) -> Option<Self::Item> {
            Some(self.gen(*index))
        }

        fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
            let end = self.len().min(start + limit);
            (start..end).map(|i| (i, self.gen(i))).collect()
        }
    }

    pub fn get() -> Shared {
        let gen = DateGenerator {
            start: Local::now(),
            end: Local::now() + Duration::days(365),
            step: Duration::seconds(999),
        };
        // gen
        let filter = SimpleCaseInsensitiveFilter::new("");
        Rc::new(FilteredList::new(gen, filter))
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let r = UpdateHandle::new();
    let selection_mode = make_widget! {
        #[layout(right)]
        #[handler(msg = SelectionMode)]
        struct {
            #[widget] _ = Label::new("Selection:"),
            #[widget] _ = RadioBox::new_msg("none", r, SelectionMode::None).with_state(true),
            #[widget] _ = RadioBox::new_msg("single", r, SelectionMode::Single),
            #[widget] _ = RadioBox::new_msg("multiple", r, SelectionMode::Multiple),
        }
    };

    let data = data::get();
    println!("filter-list: {} entries", data.len());
    let data2 = data.clone();
    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[layout(down)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(handler = set_selection_mode)] _ = selection_mode,
                #[widget] filter = EditBox::new("").on_edit(move |text, mgr| {
                    let update = data2
                        .set_filter(SimpleCaseInsensitiveFilter::new(text));
                    mgr.trigger_update(update, 0);
                    None
                }),
                #[widget(handler = select)] list: ScrollBars<ListView<Down, data::Shared>> =
                    ScrollBars::new(ListView::new(data)),
            }
            impl {
                fn set_selection_mode(
                    &mut self,
                    mgr: &mut Manager,
                    mode: SelectionMode
                ) -> Response<VoidMsg> {
                    *mgr |= self.list.set_selection_mode(mode);
                    Response::None
                }
                fn select(
                    &mut self,
                    _: &mut Manager,
                    msg: ListMsg<usize, VoidMsg>,
                ) -> Response<VoidMsg> {
                    println!("Selection message: {:?}", msg);
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
