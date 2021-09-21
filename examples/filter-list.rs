// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::dir::Down;
use kas::event::ChildMsg;
use kas::prelude::*;
use kas::updatable::ListData;
use kas::widgets::view::{driver, ListView, SelectionMode, SingleView};
use kas::widgets::{EditBox, Label, RadioBox, ScrollBars, Window};

mod data {
    use kas::updatable::filter::{ContainsCaseInsensitive, FilteredList};
    use std::rc::Rc;

    type SC = &'static [&'static str];
    pub type Shared = Rc<FilteredList<SC, ContainsCaseInsensitive>>;

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
        let filter = ContainsCaseInsensitive::new("");
        Rc::new(FilteredList::new(MONTHS.into(), filter))
    }
}

fn main() -> Result<(), kas::shell::Error> {
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
    let filter = data.filter.clone();
    let filter_driver = driver::Widget::<EditBox>::default();

    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[layout(down)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(use_msg = set_selection_mode)] _ = selection_mode,
                #[widget] filter = SingleView::new_with_driver(filter_driver, filter),
                #[widget(use_msg = select)] list:
                    ScrollBars<ListView<Down, data::Shared, driver::DefaultNav>> =
                    ScrollBars::new(ListView::new(data)),
            }
            impl {
                fn set_selection_mode(&mut self, mgr: &mut Manager, mode: SelectionMode) {
                    *mgr |= self.list.set_selection_mode(mode);
                }
                fn select(&mut self, _: &mut Manager, msg: ChildMsg<usize, VoidMsg>) {
                    println!("Selection message: {:?}", msg);
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
