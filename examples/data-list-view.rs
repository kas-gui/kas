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
use kas::row;
use kas::view::{ListView, ListViewGuard};
use kas::widget::adapter::WithAny;
use kas::widget::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Control {
    None,
    SetLen(usize),
    DecrLen,
    IncrLen,
    Select(usize),
    Update(usize, String),
}

#[derive(Clone, Debug)]
struct ReverseList;

#[derive(Debug)]
struct MyData {
    len: usize,
    active: usize,
    active_string: String,
    strings: HashMap<usize, String>,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            len,
            active: 0,
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
            Control::Select(index) => {
                if index != self.active {
                    self.active = index;
                    self.active_string = self.get(index);
                }
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
impl SharedData for MyData {
    type Key = usize;
    type Item = (bool, String);
    type ItemRef<'b> = Self::Item;

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
        let index = *key;
        let is_active = self.active == index;
        let text = self.get(index);
        Some((is_active, text))
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

type Data = (bool, String);

struct MyDriver;
impl ListViewGuard<MyData> for MyDriver {
    type Widget = Box<dyn Widget<Data = Data>>;

    fn make(&mut self, key: &usize) -> Self::Widget {
        let index = *key;
        let label = label(format!("Entry number {}", index + 1));
        let radio = RadioButton::new_msg(
            "display this entry",
            |data: &Data| data.0,
            move || Control::Select(index),
        );
        Box::new(kas::column![
            row![WithAny::new(label), radio],
            EditBox::string(
                |data: &Data| data.1.clone(),
                move |string| Control::Update(index, string.to_string()),
            ),
        ])
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
            TextButton::new_msg("↓↑", ReverseList),
        ]),
    ];

    type MyList = ListView<MyData, MyDriver, Direction>;
    let list = ListView::new_with_direction(Direction::Down, MyDriver);

    let ui = singleton! {
        #[widget{
            layout = column! [
                "Demonstration of dynamic widget creation / deletion",
                self.controls,
                "Contents of selected entry:",
                self.display,
                Separator::new(),
                self.list,
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget(&self.num)] controls: impl Widget<Data = usize> = controls,
            #[widget(&self.data)] display: impl Widget<Data = MyData> = Text::new(|data: &MyData| data.active_string.to_string()),
            #[widget(&self.data)] list: ScrollBars<MyList> =
                ScrollBars::new(list).with_fixed_bars(false, true),
            num: usize = 3,
            data: MyData = MyData::new(3),
        }
        impl Events for Self {
            type Data = ();

            fn handle_messages(&mut self, data: &Self::Data, mgr: &mut EventMgr) {
                if let Some(control) = mgr.try_pop() {
                    self.data.handle(control);
                    mgr.update(self.as_node_mut(data));
                } else if let Some(_) = mgr.try_pop::<ReverseList>() {
                    let dir = self.list.direction().reversed();
                    *mgr |= self.list.set_direction(dir);
                }
            }
        }
    };
    let window = Window::new(ui, "Dynamic widget demo");

    let theme = kas::theme::FlatTheme::new();
    kas::shell::DefaultShell::new((), theme)?
        .with(window)?
        .run()
}
