// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::dir::Down;
use kas::prelude::*;
use kas::updatable::filter::ContainsCaseInsensitive;
use kas::widgets::view::{self, driver, SelectionMode, SelectionMsg, SingleView};
use kas::widgets::{EditBox, Label, RadioBox, RadioBoxGroup, ScrollBars, Window};

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
    let selection_mode = make_widget! {
        #[widget{
            layout = list(right): *;
        }]
        struct {
            #[widget] _ = Label::new("Selection:"),
            #[widget] _ = RadioBox::new_msg("none", r.clone(), SelectionMode::None).with_state(true),
            #[widget] _ = RadioBox::new_msg("single", r.clone(), SelectionMode::Single),
            #[widget] _ = RadioBox::new_msg("multiple", r, SelectionMode::Multiple),
        }
    };

    let data = MONTHS;
    println!("filter-list: {} entries", data.len());
    let filter = ContainsCaseInsensitive::new("");
    let filter_driver = driver::Widget::<EditBox>::default();
    type FilteredList = view::FilteredList<&'static [&'static str], ContainsCaseInsensitive>;
    type ListView = view::ListView<Down, FilteredList, driver::DefaultNav>;
    let filtered = FilteredList::new(data, filter.clone());

    let widget = make_widget! {
        #[widget{
            layout = list(down): *;
        }]
        struct {
            #[widget] _ = selection_mode,
            #[widget] filter = SingleView::new_with_driver(filter_driver, filter),
            #[widget] list: ScrollBars<ListView> =
                ScrollBars::new(ListView::new(filtered)),
        }
        impl Handler for Self {
            fn on_message(&mut self, mgr: &mut EventMgr, _: usize) -> Response {
                if let Some(mode) = mgr.try_pop_msg() {
                    *mgr |= self.list.set_selection_mode(mode);
                } else if let Some(msg) = mgr.try_pop_msg::<SelectionMsg<usize>>() {
                    println!("Selection message: {:?}", msg);
                }
                Response::Unused
            }
        }
    };
    let window = Window::new("Filter-list", widget);

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
