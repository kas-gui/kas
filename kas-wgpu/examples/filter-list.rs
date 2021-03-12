// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter list example

use kas::data::{ListData, SimpleCaseInsensitiveFilter};
use kas::dir::Down;
use kas::event::UpdateHandle;
use kas::prelude::*;
use kas::widget::view::{ListMsg, ListView, SelectionMode};
use kas::widget::{EditBox, Label, RadioBox, ScrollBars, Window};

mod data {
    use kas::data::{FilteredList, SimpleCaseInsensitiveFilter};
    use std::rc::Rc;

    type SC = &'static [&'static str];
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
