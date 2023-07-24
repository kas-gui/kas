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
//! 50k entries (200k widgets) has slow init and resize but most interaction
//! is still fast.

use kas::prelude::*;
use kas::row;
use kas::widget::adapter::WithAny;
use kas::widget::edit::{EditBox, EditField, EditGuard};
use kas::widget::{
    Adapt, Label, List, RadioButton, ScrollBarRegion, Separator, StringLabel, Text, TextButton,
};

#[derive(Debug)]
struct SelectEntry(usize);

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
    active_string: String,
}
impl Data {
    fn handle(&mut self, control: Control) {
        let len = match control {
            Control::None => return,
            Control::SetLen(len) => len,
            Control::DecrLen => self.len.saturating_sub(1),
            Control::IncrLen => self.len.saturating_add(1),
            Control::Reverse => {
                self.dir = self.dir.reversed();
                return;
            }
            Control::Select(index, text) => {
                self.active = index;
                self.active_string = text;
                return;
            }
            Control::UpdateCurrent(text) => {
                self.active_string = text.clone();
                return;
            }
        };

        self.len = len;
        if self.active >= len && len > 0 {
            self.active = len - 1;
            // NOTE: We should update self.active_string here but we cannot
            // access the newly active widget's data from here.
        }
    }
}

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    type Data = Data;

    fn activate(edit: &mut EditField<Self>, _: &Data, cx: &mut EventMgr) -> Response {
        cx.push(SelectEntry(edit.guard.0));
        Response::Used
    }

    fn edit(edit: &mut EditField<Self>, data: &Data, cx: &mut EventMgr) {
        if data.active == edit.guard.0 {
            cx.push(Control::UpdateCurrent(edit.get_string()));
        }
    }
}

impl_scope! {
    // The list entry
    #[widget{
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
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }

    impl Events for Self {
        type Data = Data;

        fn handle_messages(&mut self, data: &Data, cx: &mut EventMgr) {
            if let Some(SelectEntry(n)) = cx.try_pop() {
                if data.active != n {
                    cx.push(Control::Select(n, self.edit.get_string()));
                }
            }
        }
    }

    impl Self {
        fn new(n: usize) -> Self {
            ListEntry {
                core: Default::default(),
                label: Label::new(format!("Entry number {}", n + 1)),
                radio: RadioButton::new_msg(
                    "display this entry",
                    move |active| *active == n,
                    move || SelectEntry(n),
                ),
                edit: EditBox::new(ListEntryGuard(n)).with_text(format!("Entry #{}", n + 1)),
            }
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = row![
        "Number of rows:",
        EditBox::parser(|n| *n, Control::SetLen),
        WithAny::new(row![
            // This button is just a click target; it doesn't do anything!
            TextButton::new_msg("Set", Control::None),
            TextButton::new_msg("−", Control::DecrLen),
            TextButton::new_msg("+", Control::IncrLen),
            TextButton::new_msg("↓↑", Control::Reverse),
        ]),
    ];

    let entries = vec![ListEntry::new(0), ListEntry::new(1), ListEntry::new(2)];
    let data = Data {
        len: entries.len(),
        active: 0,
        dir: Direction::Down,
        active_string: entries[0].label.get_string(),
    };

    let list = List::new_dir_vec(data.dir, entries).on_update(|cx, list, data| {
        *cx |= list.set_direction(data.dir);
        let len = data.len;
        if len != list.len() {
            list.resize_with(data, cx, len, ListEntry::new);
        }
    });
    let tree = kas::column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &Data| &data.len),
        "Contents of selected entry:",
        Text::new(|data: &Data| data.active_string.to_string()),
        Separator::new(),
        ScrollBarRegion::new(list).with_fixed_bars(false, true),
    ];

    let ui = Adapt::new(tree, data).on_message(|_, data, control| data.handle(control));

    let window = Window::new(ui, "Dynamic widget demo");

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
