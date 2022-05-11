// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::dir::Down;
use kas::prelude::*;
use kas::updatable::{filter::ContainsCaseInsensitive, SingleData};
use kas::widgets::view::{self, driver, SelectionMode, SelectionMsg};
use kas::widgets::{EditBox, RadioBox, RadioBoxGroup, ScrollBars, Window};

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

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let r = RadioBoxGroup::default();

    let data = MONTHS;
    println!("filter-list: {} entries", data.len());
    let filter = ContainsCaseInsensitive::new("");
    type FilteredList = view::FilteredList<&'static [&'static str], ContainsCaseInsensitive>;
    type ListView = view::ListView<Down, FilteredList, driver::DefaultNav>;
    let filtered = FilteredList::new(data, filter.clone());

    let widget = impl_singleton! {
        #[widget{
            layout = column: [
                row: ["Selection:", self.r0, self.r1, self.r2],
                self.filter,
                self.list,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] r0 = RadioBox::new_msg("none", r.clone(), SelectionMode::None).with_state(true),
            #[widget] r1 = RadioBox::new_msg("single", r.clone(), SelectionMode::Single),
            #[widget] r2 = RadioBox::new_msg("multiple", r, SelectionMode::Multiple),
            #[widget] filter = EditBox::new("")
                .on_edit(move |s, mgr| filter.update(mgr, s.to_string())),
            #[widget] list: ScrollBars<ListView> =
                ScrollBars::new(ListView::new(filtered)),
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(mode) = mgr.try_pop_msg() {
                    *mgr |= self.list.set_selection_mode(mode);
                } else if let Some(msg) = mgr.try_pop_msg::<SelectionMsg<usize>>() {
                    println!("Selection message: {:?}", msg);
                }
            }
        }
    };
    let window = Window::new("Filter-list", widget);

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
