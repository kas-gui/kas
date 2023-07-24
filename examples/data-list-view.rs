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
use kas::row;
use kas::view::{Driver, ListData, ListView, SharedData};
use kas::widget::adapter::WithAny;
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
    Update(usize, String),
}

#[derive(Debug)]
struct Data {
    // Using one verson for the whole data structure forces reloading all items
    // whenever the version changes, but this is fine in our example.
    ver: u64,
    len: usize,
    active: usize,
    dir: Direction,
    active_string: String,
    strings: HashMap<usize, String>,
}
impl Data {
    fn new(len: usize) -> Self {
        Data {
            ver: 0,
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
        self.ver += 1;
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
            self.active_string = self.get(self.active);
        }
    }
}

type Item = (usize, String); // (active index, entry's text)

#[derive(Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    type Data = Item;

    fn update(edit: &mut EditField<Self>, data: &Item, cx: &mut ConfigMgr) {
        if !edit.has_key_focus() {
            *cx |= edit.set_string(data.1.clone());
        }
    }

    fn activate(edit: &mut EditField<Self>, _: &Item, cx: &mut EventMgr) -> Response {
        cx.push(SelectEntry(edit.guard.0));
        Response::Used
    }

    fn edit(edit: &mut EditField<Self>, _: &Item, cx: &mut EventMgr) {
        cx.push(Control::Update(edit.guard.0, edit.get_string()));
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
        #[widget]
        radio: RadioButton<Item>,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }

    impl Events for Self {
        type Data = Item;

        fn handle_messages(&mut self, data: &Item, cx: &mut EventMgr) {
            if let Some(SelectEntry(n)) = cx.try_pop() {
                if data.0 != n {
                    cx.push(Control::Select(n, self.edit.get_string()));
                }
            }
        }
    }
}

// Once RPITIT is stable we can replace this with range + map
struct KeyIter {
    start: usize,
    end: usize,
    ver: u64,
}
impl Iterator for KeyIter {
    type Item = (usize, u64);
    fn next(&mut self) -> Option<Self::Item> {
        let mut item = None;
        if self.start < self.end {
            item = Some((self.start, self.ver));
            self.start += 1;
        }
        item
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.end.saturating_sub(self.start);
        (len, Some(len))
    }
}

impl SharedData for Data {
    type Key = usize;
    type Version = u64;
    type Item = Item;
    type ItemRef<'b> = Item;

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }
    fn borrow(&self, key: &Self::Key) -> Option<Item> {
        Some((self.active, self.get(*key)))
    }
}
impl ListData for Data {
    type KeyIter<'b> = KeyIter;

    fn len(&self) -> usize {
        self.len
    }

    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
        KeyIter {
            start: start.min(self.len),
            end: (start + limit).min(self.len),
            ver: self.ver,
        }
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
                move |data: &Item| data.0 == n,
                move || SelectEntry(n),
            ),
            edit: EditBox::new(ListEntryGuard(n)),
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

    let data = Data::new(3);

    let list = ListView::new_with_direction(data.dir, MyDriver).on_update(|cx, list, data| {
        *cx |= list.set_direction(data.dir);
    });
    let tree = kas::column![
        "Demonstration of dynamic widget creation / deletion",
        controls.map(|data: &Data| &data.len),
        "Contents of selected entry:",
        Text::new(|data: &Data| data.active_string.clone()),
        Separator::new(),
        ScrollBars::new(list).with_fixed_bars(false, true),
    ];

    let ui = Adapt::new(tree, data).on_message(|_, data, control| data.handle(control));

    let window = Window::new(ui, "Dynamic widget demo");

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
