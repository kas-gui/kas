// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data list example (direct representation)
//!
//! Objective: test performance using a naive list design; stress test with
//! ridiculous numbers of widgets.
//!
//! Compare: `data-list-view.rs` has the same functionality but with a dynamic
//! view, and thus scales *much* better to large numbers of rows.
//!
//! Results (debug build): 1000 entries requires approx 1s for init and may
//! have a small delay on resize but is otherwise very fast.
//!
//! Results (release build): 1k entries is fast, 10k has some noticable lag
//! (changing the list length and resizing).
//! 250k entries (1M widgets) has very slow init and resize but most interaction
//! is still fast; Widget::update (broadcast) causes barely noticable lag.

use kas::prelude::*;
use kas::widget::*;

#[derive(Debug)]
struct SelectEntry;

#[derive(Clone, Debug)]
enum Control {
    None,
    SetLen(usize),
    DecrLen,
    IncrLen,
    Reverse,
    Select(usize, String),
    UpdateCurrent(String),
}

#[derive(Debug)]
struct Data {
    len: usize,
    active: usize,
    dir: Direction,
    current: String,
}

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard<Data> for ListEntryGuard {
    fn activate(_edit: &mut EditField<Data, Self>, cx: &mut EventCx<Data>) -> Response {
        cx.push(SelectEntry);
        Response::Used
    }

    fn edit(edit: &mut EditField<Data, Self>, cx: &mut EventCx<Data>) {
        if cx.data().active == edit.guard.0 {
            cx.push(Control::UpdateCurrent(edit.get_string()));
        }
    }
}

impl_scope! {
    // The list entry
    #[derive(Debug)]
    #[widget{
        data = Data;
        layout = column! [
            row! [self.label, self.radio],
            self.edit,
        ];
    }]
    struct ListEntry {
        core: widget_core!(),
        #[widget(&())]
        label: StringLabel,
        #[widget(&data.active)]
        radio: RadioButton<usize>,
        // We deliberately use these widgets to store state instead of passing.
        // See examples/data-list-view.rs for a better option.
        #[widget]
        edit: EditBox<Data, ListEntryGuard>,
    }
}

impl ListEntry {
    fn new(n: usize) -> Self {
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioButton::new_msg(
                "display this entry",
                move |active| *active == n,
                move || SelectEntry,
            ),
            edit: EditBox::new(format!("Entry #{}", n + 1)).with_guard(ListEntryGuard(n)),
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = kas::row![
        "Number of rows:",
        EditBox::parser(|n| *n, Control::SetLen),
        // This button is just a click target; it doesn't do anything!
        button("Set", Control::None),
        button("−", Control::DecrLen),
        button("+", Control::IncrLen),
        button("↓↑", Control::Reverse),
    ];

    let entries = vec![ListEntry::new(0), ListEntry::new(1), ListEntry::new(2)];
    let data = Data {
        len: entries.len(),
        active: 0,
        dir: Direction::Down,
        current: entries[0].label.get_string(), //"Entry #1".to_string(),
    };

    let list = List::new_dir_vec(data.dir, entries)
        .on_messages(|list, cx, n| {
            if let Some(SelectEntry) = cx.try_pop() {
                if cx.data().active != n {
                    cx.push(Control::Select(n, list[n].edit.get_string()));
                }
            }
        })
        .on_update(|list, cx| {
            *cx |= list.set_direction(cx.data().dir);
            let len = cx.data().len;
            if len != list.len() {
                list.resize_with(cx, len, ListEntry::new);
            }
        });
    let tree = kas::column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &Data| &data.len),
        "Contents of selected entry:",
        Text::new(|data: &Data| data.current.to_string()),
        Separator::new(),
        ScrollBarRegion::new(list).with_fixed_bars(false, true),
    ];

    let adapt =
        Adapt::new(tree, data, |_, data| data).on_message(|_, data, control| match control {
            Control::None => return,
            Control::SetLen(len) => {
                data.len = len;
            }
            Control::DecrLen => {
                data.len = data.len.saturating_sub(1);
            }
            Control::IncrLen => {
                data.len = data.len.saturating_add(1);
            }
            Control::Reverse => {
                data.dir = data.dir.reversed();
            }
            Control::Select(n, text) => {
                data.active = n;
                data.current = text;
            }
            Control::UpdateCurrent(text) => {
                data.current = text;
            }
        });

    let window = dialog::Window::new("Dynamic widget demo", adapt);

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new(theme)?.with(window)?.run()
}
