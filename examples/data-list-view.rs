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

use kas::model::*;
use kas::prelude::*;
use kas::view::{ListView, ListViewGuard};
use kas::widget::*;
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
    UpdateCurrent(String),
}

#[derive(Debug)]
struct MyData {
    len: usize,
    active: usize,
    dir: Direction,
    active_string: String,
    strings: HashMap<usize, String>,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            len,
            active: 0,
            dir: Direction::Down,
            active_string: String::from("Entry 1"),
            strings: HashMap::new(),
        }
    }
    fn get(&self, index: usize) -> String {
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
            Control::UpdateCurrent(text) => {
                self.active_string = text.clone();
                self.strings.insert(self.active, text);
                return;
            }
        };

        self.len = len;
        if self.active >= len && len > 0 {
            self.active = len - 1;
            self.active_string = self.get(self.active);
        }
    }
}

type Item = (usize, String); // (active index, entry's text)

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    type Data = Item;

    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx<Item>) -> Response {
        cx.push(SelectEntry(edit.guard.0));
        Response::Used
    }

    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx<Item>) {
        if cx.data().0 == edit.guard.0 {
            cx.push(Control::UpdateCurrent(edit.get_string()));
        }
    }
}

impl_scope! {
    // The list entry
    #[derive(Debug)]
    #[widget{
        data = Item;
        layout = column! [
            row! [self.label, self.radio],
            self.edit,
        ];
    }]
    struct ListEntry {
        core: widget_core!(),
        #[widget(&())]
        label: StringLabel,
        #[widget]
        radio: RadioButton<Item>,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }
    impl Widget for Self {
        fn handle_messages(&mut self, cx: &mut EventCx<Self::Data>) {
            if let Some(SelectEntry(n)) = cx.try_pop() {
                if cx.data().0 != n {
                    cx.push(Control::Select(n, self.edit.get_string()));
                }
            }
        }
    }
}

impl SharedData for MyData {
    type Key = usize;
    type Item = Item;
    type ItemRef<'b> = Item;

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }
    fn borrow(&self, key: &Self::Key) -> Option<Item> {
        Some((self.active, self.get(*key)))
    }
}
impl ListData for MyData {
    type KeyIter<'b> = std::ops::Range<usize>;

    fn len(&self) -> usize {
        self.len
    }

    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
        start.min(self.len)..(start + limit).min(self.len)
    }
}

#[derive(Debug)]
struct MyDriver;
impl ListViewGuard<MyData> for MyDriver {
    type Widget = ListEntry;

    fn make(&mut self, key: &usize) -> Self::Widget {
        let n = *key;
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioButton::new_msg(
                "display this entry",
                move |data: &Item| data.0 == n,
                move || SelectEntry(n),
            ),
            edit: EditBox::new(ListEntryGuard(n)),
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

    let data = MyData::new(3);

    let list = ListView::new_with_direction(data.dir, MyDriver).on_update(|list, cx| {
        *cx |= list.set_direction(cx.data().dir);
    });
    let ui = kas::column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &MyData| &data.len),
        "Contents of selected entry:",
        Text::new(|data: &MyData| data.active_string.clone()),
        Separator::new(),
        ScrollBars::new(list).with_fixed_bars(false, true),
    ];

    let adapt =
        Adapt::new(ui, data, |_, data| data).on_message(|_, data, control| data.handle(control));

    let window = dialog::Window::new("Dynamic widget demo", adapt);

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new(theme)?.with(window)?.run()
}
