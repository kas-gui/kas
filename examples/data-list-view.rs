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
use kas::view::clerk::{Clerk, GeneratorChanges, IndexedGenerator, Len};
use kas::view::{Driver, ListView};
use kas::widgets::{column, *};
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Control {
    SetRowLimit(bool),
    SetLen(usize),
    DecrLen,
    IncrLen,
    Reverse,
    Select(usize),
    Update(usize, String),
}

#[derive(Debug)]
struct MyData {
    row_limit: bool,
    len: usize,
    last_change: GeneratorChanges<usize>,
    last_key: usize,
    active: usize,
    dir: Direction,
    strings: HashMap<usize, String>,
}
impl MyData {
    fn new(row_limit: bool, len: usize) -> Self {
        MyData {
            row_limit,
            len,
            last_change: GeneratorChanges::None,
            last_key: len,
            active: 0,
            dir: Direction::Down,
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
        self.last_change = GeneratorChanges::LenOnly;
        let len = match control {
            Control::SetRowLimit(row_limit) => {
                self.row_limit = row_limit;
                return;
            }
            Control::SetLen(len) => len,
            Control::DecrLen => self.len.saturating_sub(1),
            Control::IncrLen => self.len.saturating_add(1),
            Control::Reverse => {
                self.last_change = GeneratorChanges::None;
                self.dir = self.dir.reversed();
                return;
            }
            Control::Select(index) => {
                self.last_change = GeneratorChanges::Any;
                self.active = index;
                return;
            }
            Control::Update(index, text) => {
                self.last_change = GeneratorChanges::Range(index..index + 1);
                self.last_key = self.last_key.max(index);
                self.strings.insert(index, text);
                return;
            }
        };

        self.len = len;
        if self.active >= len && len > 0 {
            self.active = len - 1;
        }
    }
}

type MyItem = (usize, String); // (active index, entry's text)

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    type Data = MyItem;

    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &MyItem) {
        if !edit.has_edit_focus() {
            edit.set_string(cx, data.1.to_string());
        }
    }

    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, _: &MyItem) -> IsUsed {
        cx.push(Control::Select(edit.guard.0));
        Used
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &MyItem) {
        cx.push(Control::Update(edit.guard.0, edit.clone_string()));
    }
}

#[impl_self]
mod ListEntry {
    // The list entry
    #[widget]
    #[layout(column! [
        row! [self.label, self.radio],
        self.edit,
    ])]
    struct ListEntry {
        core: widget_core!(),
        #[widget(&())]
        label: Label<String>,
        #[widget]
        radio: RadioButton<MyItem>,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }

    impl Events for Self {
        type Data = MyItem;
    }
}

struct ListEntryDriver;
impl Driver<usize, MyItem> for ListEntryDriver {
    const TAB_NAVIGABLE: bool = true;

    type Widget = ListEntry;

    fn make(&mut self, key: &usize) -> Self::Widget {
        let n = *key;
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioButton::new_msg(
                "display this entry",
                move |_, data: &MyItem| data.0 == n,
                move || Control::Select(n),
            ),
            edit: EditBox::new(ListEntryGuard(n)),
        }
    }

    fn navigable(_: &Self::Widget) -> bool {
        false
    }
}

#[derive(Default)]
struct Generator;
impl Clerk<usize> for Generator {
    type Data = MyData;
    type Item = MyItem;

    fn len(&self, data: &Self::Data, lbound: usize) -> Len<usize> {
        if data.row_limit {
            Len::Known(data.len)
        } else {
            Len::LBound((data.active.max(data.last_key) + 1).max(lbound))
        }
    }
}
impl IndexedGenerator<usize> for Generator {
    fn update(&mut self, data: &Self::Data) -> GeneratorChanges<usize> {
        // We assume that `MyData::handle` has only been called once since this
        // method was last called.
        data.last_change.clone()
    }

    fn generate(&self, data: &Self::Data, index: usize) -> Self::Item {
        (data.active, data.get_string(index))
    }
}

fn main() -> kas::runner::Result<()> {
    env_logger::init();

    let controls = row![
        "Number of rows:",
        EditBox::parser(|n| *n, Control::SetLen),
        row![
            // This button is just a click target; it doesn't do anything!
            Button::label("Set"),
            Button::label_msg("−", Control::DecrLen),
            Button::label_msg("+", Control::IncrLen),
            Button::label_msg("↓↑", Control::Reverse),
        ]
        .map_any(),
    ];

    let data = MyData::new(false, 5);

    let clerk = Generator::default();
    let list = ListView::new(clerk, ListEntryDriver).on_update(|cx, list, data: &MyData| {
        list.set_direction(cx, data.dir);
    });
    let tree = column![
        "Demonstration of dynamic widget creation / deletion",
        CheckButton::new("Explicit row limit:", |_, data: &MyData| data.row_limit)
            .with_msg(Control::SetRowLimit),
        controls
            .map(|data: &MyData| &data.len)
            .on_update(|cx, _, data: &MyData| cx.set_disabled(!data.row_limit)),
        "Contents of selected entry:",
        Text::new_gen(|_, data: &MyData| data.get_string(data.active)),
        Separator::new(),
        ScrollRegion::new_over_viewport(list).with_fixed_bars(false, true),
    ];

    let ui = tree
        .with_state(data)
        .on_message(|_, data, control| data.handle(control));

    let window = Window::new(ui, "Dynamic widget demo");

    kas::runner::Runner::new(())?.with(window).run()
}
