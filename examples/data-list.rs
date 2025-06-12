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
use kas::widgets::edit::{EditBox, EditField, EditGuard};
use kas::widgets::{column, row};
use kas::widgets::{Button, Label, List, RadioButton, ScrollBarRegion, Separator, Text};

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

    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Data) -> IsUsed {
        cx.push(SelectEntry(edit.guard.0));
        Used
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Data) {
        if data.active == edit.guard.0 {
            cx.push(Control::UpdateCurrent(edit.clone_string()));
        }
    }
}

#[impl_self]
mod ListEntry {
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
        label: Label<String>,
        #[widget(&data.active)]
        radio: RadioButton<usize>,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }

    impl Events for Self {
        type Data = Data;

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Data) {
            if let Some(SelectEntry(n)) = cx.try_pop() {
                if data.active != n {
                    cx.push(Control::Select(n, self.edit.clone_string()));
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
                    move |_, active| *active == n,
                    move || SelectEntry(n),
                ),
                edit: EditBox::new(ListEntryGuard(n)).with_text(format!("Entry #{}", n + 1)),
            }
        }
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let controls = row![
        "Number of rows:",
        EditBox::parser(|n| *n, Control::SetLen),
        row![
            // This button is just a click target; it doesn't do anything!
            Button::label_msg("Set", Control::None),
            Button::label_msg("−", Control::DecrLen),
            Button::label_msg("+", Control::IncrLen),
            Button::label_msg("↓↑", Control::Reverse),
        ]
        .map_any(),
    ];

    let data = Data {
        len: 5,
        active: 0,
        dir: Direction::Down,
        active_string: ListEntry::new(0).label.as_str().to_string(),
    };

    let list = List::new(vec![]).on_update(|cx, list, data: &Data| {
        list.set_direction(cx, data.dir);
        let len = data.len;
        if len != list.len() {
            list.resize_with(cx, data, len, ListEntry::new);
        }
    });
    let tree = column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &Data| &data.len),
        "Contents of selected entry:",
        Text::new(|_, data: &Data| data.active_string.to_string()),
        Separator::new(),
        ScrollBarRegion::new(list).with_fixed_bars(false, true),
    ];

    let ui = tree
        .with_state(data)
        .on_message(|_, data, control| data.handle(control));

    let window = Window::new(ui, "Dynamic widget demo");

    kas::runner::Runner::new(())?.with(window).run()
}
