// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data list example (indirect representation)
//!
//! This is a variant of `data-list` using the [`ListView`] widget to create a
//! dynamic view over a lazy, indirect data structure. Maximum data length is
//! thus only limited by the data types used (specifically the `i32` type used
//! to calculate the maximum scroll offset).

use kas::prelude::*;
use kas::view::{Driver, ListData, ListView, SharedData};
use kas::widgets::{column, *};
use std::collections::HashMap;

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
    Update(usize, String),
}

#[derive(Debug)]
struct Data {
    len: usize,
    active: usize,
    dir: Direction,
    active_string: String,
    strings: HashMap<usize, String>,
}
impl Data {
    fn new(len: usize) -> Self {
        Data {
            len,
            active: 0,
            dir: Direction::Down,
            active_string: String::from("Entry 1"),
            strings: HashMap::new(),
        }
    }
    fn get_string(&self, index: usize) -> String {
        self.strings
            .get(&index)
            .cloned()
            .unwrap_or_else(|| format!("Entry #{}", index + 1))
    }
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
            Control::Update(index, text) => {
                if index == self.active {
                    self.active_string = text.clone();
                }
                self.strings.insert(index, text);
                return;
            }
        };

        self.len = len;
        if self.active >= len && len > 0 {
            self.active = len - 1;
            self.active_string = self.get_string(self.active);
        }
    }
}

type Item = (usize, String); // (active index, entry's text)

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    type Data = Item;

    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &Item) {
        if !edit.has_edit_focus() {
            edit.set_string(cx, data.1.to_string());
        }
    }

    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Item) -> IsUsed {
        cx.push(SelectEntry(edit.guard.0));
        Used
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &Item) {
        cx.push(Control::Update(edit.guard.0, edit.clone_string()));
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
        label: Label<String>,
        #[widget]
        radio: RadioButton<Item>,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }

    impl Events for Self {
        type Data = Item;

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Item) {
            if let Some(SelectEntry(n)) = cx.try_pop() {
                if data.0 != n {
                    cx.push(Control::Select(n, self.edit.clone_string()));
                }
            }
        }
    }
}

impl SharedData for Data {
    type Key = usize;
    type Item = Item;

    fn get(&self, key: &Self::Key) -> Option<Item> {
        Some((self.active, self.get_string(*key)))
    }
}
impl ListData for Data {
    fn len(&self) -> usize {
        self.len
    }

    fn iter_from(&self, start: usize, limit: usize) -> impl Iterator<Item = usize> {
        let start = start.min(self.len);
        let end = (start + limit).min(self.len);
        (start..end).into_iter()
    }
}

struct MyDriver;
impl Driver<Item, Data> for MyDriver {
    type Widget = ListEntry;

    fn make(&mut self, key: &usize) -> Self::Widget {
        let n = *key;
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioButton::new_msg(
                "display this entry",
                move |_, data: &Item| data.0 == n,
                move || SelectEntry(n),
            ),
            edit: EditBox::new(ListEntryGuard(n)),
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

    let data = Data::new(5);

    let list = ListView::new(MyDriver).on_update(|cx, list, data| {
        list.set_direction(cx, data.dir);
    });
    let tree = column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &Data| &data.len),
        "Contents of selected entry:",
        Text::new(|_, data: &Data| data.active_string.clone()),
        Separator::new(),
        ScrollBars::new(list).with_fixed_bars(false, true),
    ];

    let ui = tree
        .with_state(data)
        .on_message(|_, data, control| data.handle(control));

    let window = Window::new(ui, "Dynamic widget demo");

    kas::runner::Default::new(())?.with(window).run()
}
