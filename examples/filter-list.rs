// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::dir::Down;
use kas::event::ChildMsg;
use kas::prelude::*;
use kas::updatable::filter::ContainsCaseInsensitive;
use kas::widgets::view::{driver, FilterListView, SelectionMode, SingleView};
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

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let r = RadioBoxGroup::default();
    let selection_mode = make_widget! {
        #[widget{
            layout = list(right): *;
        }]
        #[handler(msg = SelectionMode)]
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

    let window = Window::new(
        "Filter-list",
        make_widget! {
            #[widget{
                layout = list(down): *;
            }]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(use_msg = set_selection_mode)] _ = selection_mode,
                #[widget] filter = SingleView::new_with_driver(filter_driver, filter.clone()),
                #[widget(use_msg = select)] list:
                    ScrollBars<FilterListView<
                        Down,
                        &'static [&'static str],
                        ContainsCaseInsensitive,
                        driver::DefaultNav,
                    >> =
                    ScrollBars::new(FilterListView::new(data, filter)),
            }
            impl Self {
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
