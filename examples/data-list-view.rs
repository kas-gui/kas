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
use kas::view::{Driver, ListView, MaybeOwned};
use kas::widgets::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Debug)]
enum Control {
    Set(usize),
    Dir,
    Update(String),
}

#[derive(Clone, Debug)]
enum Button {
    Decr,
    Incr,
    Set,
}

#[derive(Clone, Debug)]
enum EntryMsg {
    Select,
    Update(String),
}

#[derive(Debug)]
struct MyData {
    ver: u64,
    len: usize,
    active: usize,
    strings: HashMap<usize, String>,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            ver: 1,
            len,
            active: 0,
            strings: HashMap::new(),
        }
    }
    fn get(&self, index: usize) -> String {
        self.strings
            .get(&index)
            .cloned()
            .unwrap_or_else(|| format!("Entry #{}", index + 1))
    }
}

#[derive(Debug)]
struct MySharedData {
    data: RefCell<MyData>,
}
impl MySharedData {
    fn new(len: usize) -> Self {
        MySharedData {
            data: RefCell::new(MyData::new(len)),
        }
    }
    fn set_len(&mut self, len: usize) -> Option<String> {
        let mut data = self.data.borrow_mut();
        data.ver += 1;
        data.len = len;
        if data.active >= len && len > 0 {
            data.active = len - 1;
            Some(data.get(data.active))
        } else {
            None
        }
    }
}
impl SharedData for MySharedData {
    type Key = usize;
    type Item = (bool, String);
    type ItemRef<'b> = Self::Item;

    fn version(&self) -> u64 {
        self.data.borrow().ver
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
        let index = *key;
        let data = self.data.borrow();
        let is_active = data.active == index;
        let text = data.get(index);
        Some((is_active, text))
    }
}
impl ListData for MySharedData {
    type KeyIter<'b> = std::ops::Range<usize>;

    fn len(&self) -> usize {
        self.data.borrow().len
    }
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        parent.make_child(*key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        child.next_key_after(parent)
    }

    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
        let len = self.len();
        start.min(len)..(start + limit).min(len)
    }
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
#[derive(Clone, Debug)]
struct ListEntryGuard;
impl EditGuard for ListEntryGuard {
    fn activate(_edit: &mut EditField<Self>, mgr: &mut EventMgr) -> Response {
        mgr.push_msg(EntryMsg::Select);
        Response::Used
    }

    fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        mgr.push_msg(EntryMsg::Update(edit.get_string()));
    }
}

impl_scope! {
    // The list entry
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: [
            row: [self.label, self.radio],
            self.edit,
        ];
    }]
    struct ListEntry {
        core: widget_core!(),
        #[widget]
        label: StringLabel,
        #[widget]
        radio: RadioButton,
        #[widget]
        edit: EditBox<ListEntryGuard>,
    }
}

#[derive(Debug)]
struct MyDriver {
    radio_group: RadioGroup,
}
impl Driver<(bool, String), MySharedData> for MyDriver {
    type Widget = ListEntry;

    fn make(&self) -> Self::Widget {
        // Default instances are not shown, so the data is unimportant
        ListEntry {
            core: Default::default(),
            label: Label::new(String::default()),
            radio: RadioButton::new("display this entry", self.radio_group.clone())
                .on_select(|mgr| mgr.push_msg(EntryMsg::Select)),
            edit: EditBox::new(String::default()).with_guard(ListEntryGuard),
        }
    }

    fn set_mo(
        &self,
        widget: &mut Self::Widget,
        key: &usize,
        item: MaybeOwned<(bool, String)>,
    ) -> Action {
        let label = format!("Entry number {}", *key + 1);
        let item = item.into_owned();
        widget.label.set_string(label)
            | widget.radio.set_bool(item.0)
            | widget.edit.set_string(item.1)
    }

    fn on_message(
        &self,
        mgr: &mut EventMgr,
        _: &mut Self::Widget,
        data: &MySharedData,
        key: &usize,
    ) {
        if let Some(msg) = mgr.try_pop_msg() {
            let mut borrow = data.data.borrow_mut();
            borrow.ver += 1;
            match msg {
                EntryMsg::Select => {
                    borrow.active = *key;
                }
                EntryMsg::Update(text) => {
                    borrow.strings.insert(*key, text);
                }
            }
            mgr.push_msg(Control::Update(borrow.get(borrow.active)));
            mgr.update_all(0);
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = singleton! {
        #[widget{
            layout = row: [
                "Number of rows:",
                self.edit,
                TextButton::new_msg("Set", Button::Set),
                TextButton::new_msg("−", Button::Decr),
                TextButton::new_msg("+", Button::Incr),
                TextButton::new_msg("↓↑", Control::Dir),
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] edit: EditBox<impl EditGuard> = EditBox::new("3")
                .on_afl(|mgr, text| match text.parse::<usize>() {
                    Ok(n) => mgr.push_msg(n),
                    Err(_) => (),
                }),
            n: usize = 3,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
                if index == widget_index![self.edit] {
                    if let Some(n) = mgr.try_pop_msg::<usize>() {
                        if n != self.n {
                            self.n = n;
                            mgr.push_msg(Control::Set(n))
                        }
                    }
                } else if let Some(msg) = mgr.try_pop_msg::<Button>() {
                    let n = match msg {
                        Button::Decr => self.n.saturating_sub(1),
                        Button::Incr => self.n.saturating_add(1),
                        Button::Set => self.n,
                    };
                    *mgr |= self.edit.set_string(n.to_string());
                    self.n = n;
                    mgr.push_msg(Control::Set(n));
                }
            }
        }
    };

    let driver = MyDriver {
        radio_group: RadioGroup::new(),
    };
    let data = MySharedData::new(3);
    type MyList = ListView<Direction, MySharedData, MyDriver>;
    let list = ListView::new_with_dir_driver(Direction::Down, driver, data);

    let window = singleton! {
        #[widget{
            layout = column: [
                "Demonstration of dynamic widget creation / deletion",
                self.controls,
                "Contents of selected entry:",
                self.display,
                Separator::new(),
                self.list,
            ];
        }]
        #[derive(Debug)]
        struct {
            core: widget_core!(),
            #[widget] controls = controls,
            #[widget] display: StringLabel = Label::from("Entry #1"),
            #[widget] list: ScrollBars<MyList> =
                ScrollBars::new(list).with_fixed_bars(false, true),
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(control) = mgr.try_pop_msg::<Control>() {
                    match control {
                        Control::Set(len) => {
                            if let Some(text) = self.list.data_mut().set_len(len) {
                                *mgr |= self.display.set_string(text);
                            }
                            mgr.update_all(0);
                        }
                        Control::Dir => {
                            let dir = self.list.direction().reversed();
                            *mgr |= self.list.set_direction(dir);
                        }
                        Control::Update(text) => {
                            *mgr |= self.display.set_string(text);
                        }
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Dynamic widget demo" }
        }
    };

    let theme = kas::theme::FlatTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
