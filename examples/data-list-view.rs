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

use kas::event::ChildMsg;
use kas::prelude::*;
use kas::updatable::*;
use kas::widgets::view::{Driver, ListView};
use kas::widgets::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Debug, VoidMsg)]
enum Control {
    Set(usize),
    Dir,
}

#[derive(Clone, Debug, VoidMsg)]
enum Button {
    Decr,
    Incr,
    Set,
}

#[derive(Clone, Debug, VoidMsg)]
enum EntryMsg {
    Select,
    Update(String),
}

#[derive(Debug)]
struct MyData {
    len: usize,
    // (active index, map of strings)
    data: RefCell<(usize, HashMap<usize, String>)>,
    handle: UpdateHandle,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            len,
            data: Default::default(),
            handle: UpdateHandle::new(),
        }
    }
    fn set_len(&mut self, len: usize) -> (Option<String>, UpdateHandle) {
        self.len = len;
        let mut new_text = None;
        let mut data = self.data.borrow_mut();
        if data.0 >= len && len > 0 {
            let active = len - 1;
            data.0 = active;
            drop(data);
            new_text = Some(self.get(active).1);
        }
        (new_text, self.handle)
    }
    fn get_active(&self) -> usize {
        self.data.borrow().0
    }
    // Note: in general this method should update the data source and return
    // self.handle, but for our uses this is sufficient.
    fn set_active(&mut self, active: usize) -> String {
        self.data.borrow_mut().0 = active;
        self.get(active).1
    }
    fn get(&self, index: usize) -> (bool, String) {
        let data = self.data.borrow();
        let is_active = data.0 == index;
        let text = data.1.get(&index).cloned();
        let text = text.unwrap_or_else(|| format!("Entry #{}", index + 1));
        (is_active, text)
    }
}
impl Updatable for MyData {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
impl UpdatableHandler<usize, EntryMsg> for MyData {
    fn handle(&self, key: &usize, msg: &EntryMsg) -> Option<UpdateHandle> {
        match msg {
            EntryMsg::Select => {
                self.data.borrow_mut().0 = *key;
                Some(self.handle)
            }
            EntryMsg::Update(text) => {
                self.data.borrow_mut().1.insert(*key, text.clone());
                Some(self.handle)
            }
        }
    }
}
impl ListData for MyData {
    type Key = usize;
    type Item = (usize, bool, String);

    fn len(&self) -> usize {
        self.len
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        let (is_active, text) = self.get(*key);
        Some((*key, is_active, text))
    }

    fn update(&self, _: &Self::Key, _: Self::Item) -> Option<UpdateHandle> {
        unimplemented!()
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (0..limit.min(self.len))
            .map(|n| (n, self.get_cloned(&n).unwrap()))
            .collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (start..self.len.min(start + limit))
            .map(|n| {
                let (is_active, text) = self.get(n);
                (n, (n, is_active, text))
            })
            .collect()
    }
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
#[derive(Clone, Debug)]
struct ListEntryGuard;
impl EditGuard for ListEntryGuard {
    type Msg = EntryMsg;

    fn edit(entry: &mut EditField<Self>, _: &mut Manager) -> Option<Self::Msg> {
        Some(EntryMsg::Update(entry.get_string()))
    }
}

widget! {
    // The list entry
    #[derive(Clone, Debug)]
    #[layout(column)]
    #[handler(msg=EntryMsg)]
    struct ListEntry {
        #[widget_core]
        core: CoreData,
        #[layout_data]
        layout_data: <Self as kas::LayoutData>::Data,
        #[widget]
        label: StringLabel,
        #[widget]
        radio: RadioBox<EntryMsg>,
        #[widget]
        entry: EditBox<ListEntryGuard>,
    }
}

#[derive(Debug)]
struct MyDriver {
    radio_group: UpdateHandle,
}
impl Driver<(usize, bool, String)> for MyDriver {
    type Msg = EntryMsg;
    type Widget = ListEntry;

    fn new(&self) -> Self::Widget {
        // Default instances are not shown, so the data is unimportant
        ListEntry {
            core: Default::default(),
            layout_data: Default::default(),
            label: Label::new(String::default()),
            radio: RadioBox::new("display this entry", self.radio_group)
                .on_select(move |_| Some(EntryMsg::Select)),
            entry: EditBox::new(String::default()).with_guard(ListEntryGuard),
        }
    }
    fn set(&self, widget: &mut Self::Widget, data: (usize, bool, String)) -> TkAction {
        let label = format!("Entry number {}", data.0 + 1);
        widget.label.set_string(label)
            | widget.radio.set_bool(data.1)
            | widget.entry.set_string(data.2)
    }
    fn get(&self, _widget: &Self::Widget) -> Option<(usize, bool, String)> {
        None // unused
    }
}

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let controls = make_widget! {
        #[layout(row)]
        #[handler(msg = Control)]
        struct {
            #[widget] _ = Label::new("Number of rows:"),
            #[widget(map_msg = activate)] edit: impl HasString = EditBox::new("3")
                .on_afl(|text, _| text.parse::<usize>().ok()),
            #[widget(map_msg = button)] _ = TextButton::new_msg("Set", Button::Set),
            #[widget(map_msg = button)] _ = TextButton::new_msg("−", Button::Decr),
            #[widget(map_msg = button)] _ = TextButton::new_msg("+", Button::Incr),
            #[widget] _ = TextButton::new_msg("↓↑", Control::Dir),
            n: usize = 3,
        }
        impl {
            fn activate(&mut self, _: &mut Manager, n: usize) -> Control {
                self.n = n;
                Control::Set(n)
            }
            fn button(&mut self, mgr: &mut Manager, msg: Button) -> Control {
                let n = match msg {
                    Button::Decr => self.n.saturating_sub(1),
                    Button::Incr => self.n.saturating_add(1),
                    Button::Set => self.n,
                };
                *mgr |= self.edit.set_string(n.to_string());
                self.n = n;
                Control::Set(n)
            }
        }
    };

    let driver = MyDriver {
        radio_group: UpdateHandle::new(),
    };
    let data = MyData::new(3);
    type MyList = ListView<Direction, MyData, MyDriver>;
    let list = ListView::new_with_dir_driver(Direction::Down, driver, data);

    let window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget(use_msg = control)] controls -> Control = controls,
                #[widget] _ = Label::new("Contents of selected entry:"),
                #[widget] display: StringLabel = Label::from("Entry #0"),
                #[widget] _ = Separator::new(),
                #[widget(use_msg = set_radio)] list: ScrollBars<MyList> =
                    ScrollBars::new(list).with_bars(false, true),
            }
            impl {
                fn control(&mut self, mgr: &mut Manager, control: Control) {
                    match control {
                        Control::Set(len) => {
                            let (opt_text, handle) = self.list.data_mut().set_len(len);
                            if let Some(text) = opt_text {
                                *mgr |= self.display.set_string(text);
                            }
                            mgr.trigger_update(handle, 0);
                        }
                        Control::Dir => {
                            let dir = self.list.direction().reversed();
                            *mgr |= self.list.set_direction(dir);
                        }
                    }
                }
                fn set_radio(&mut self, mgr: &mut Manager, msg: ChildMsg<usize, EntryMsg>) {
                    match msg {
                        ChildMsg::Select(_) | ChildMsg::Deselect(_) => (),
                        ChildMsg::Child(n, EntryMsg::Select) => {
                            let text = self.list.data_mut().set_active(n);
                            *mgr |= self.display.set_string(text);
                        }
                        ChildMsg::Child(n, EntryMsg::Update(text)) => {
                            if n == self.list.data().get_active() {
                                *mgr |= self.display.set_string(text);
                            }
                        }
                    }
                }
            }
        },
    );

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
