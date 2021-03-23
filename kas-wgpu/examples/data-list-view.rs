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

use kas::data::*;
use kas::event::ChildMsg;
use kas::prelude::*;
use kas::widget::view::{Driver, ListView};
use kas::widget::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Debug, VoidMsg)]
enum Control {
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
    active: usize,
    map: RefCell<HashMap<usize, (bool, String)>>,
    handle: UpdateHandle,
}
impl MyData {
    fn new(len: usize) -> Self {
        MyData {
            len,
            active: 0,
            map: Default::default(),
            handle: UpdateHandle::new(),
        }
    }
    fn set_len(&mut self, len: usize) -> (Option<String>, UpdateHandle) {
        self.len = len;
        let mut new_text = None;
        if self.active >= len && len > 0 {
            if let Some(value) = self.map.get_mut().get_mut(&self.active) {
                value.0 = false;
            }
            self.active = len - 1;
            if let Some(value) = self.map.get_mut().get_mut(&self.active) {
                value.0 = true;
            }
            new_text = Some(self.get(self.active).1);
        }
        (new_text, self.handle)
    }
    fn get_active(&self) -> usize {
        self.active
    }
    // Note: in general this method should update the data source and return
    // self.handle, but for our uses this is sufficient.
    fn set_active(&mut self, active: usize) -> String {
        self.active = active;
        self.get(active).1
    }
    fn get(&self, n: usize) -> (bool, String) {
        self.map
            .borrow()
            .get(&n)
            .cloned()
            .unwrap_or_else(|| (n == self.active, format!("Entry #{}", n + 1)))
    }
}
impl Updatable for MyData {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
impl RecursivelyUpdatable for MyData {}
impl ListData for MyData {
    type Key = usize;
    type Item = (bool, String);

    fn len(&self) -> usize {
        self.len
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        Some(self.get(*key))
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        self.map.borrow_mut().insert(key.clone(), value);
        Some(self.handle)
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (0..limit.min(self.len))
            .map(|n| (n, self.get_cloned(&n).unwrap()))
            .collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (start..self.len.min(start + limit))
            .map(|n| (n, self.get(n)))
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

// The list entry
#[derive(Clone, Debug, Widget)]
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

#[derive(Debug)]
struct MyDriver {
    radio_group: UpdateHandle,
}
impl Driver<usize, (bool, String)> for MyDriver {
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
    fn set(&self, widget: &mut Self::Widget, n: usize, data: (bool, String)) -> TkAction {
        widget.label.set_string(format!("Entry number {}", n + 1))
            | widget.radio.set_bool(data.0)
            | widget.entry.set_string(data.1)
    }
    fn get(&self, widget: &Self::Widget, _: &usize) -> Option<(bool, String)> {
        let b = widget.radio.get_bool();
        let s = widget.entry.get_string();
        Some((b, s))
    }
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let controls = make_widget! {
        #[layout(row)]
        #[handler(msg = usize)]
        struct {
            #[widget] _ = Label::new("Number of rows:"),
            #[widget(handler = activate)] edit: impl HasString = EditBox::new("3")
                .on_afl(|text, _| text.parse::<usize>().ok()),
            #[widget(handler = button)] _ = TextButton::new_msg("Set", Control::Set),
            #[widget(handler = button)] _ = TextButton::new_msg("−", Control::Decr),
            #[widget(handler = button)] _ = TextButton::new_msg("+", Control::Incr),
            n: usize = 3,
        }
        impl {
            fn activate(&mut self, _: &mut Manager, n: usize) -> Response<usize> {
                self.n = n;
                n.into()
            }
            fn button(&mut self, mgr: &mut Manager, msg: Control) -> Response<usize> {
                let n = match msg {
                    Control::Decr => self.n.saturating_sub(1),
                    Control::Incr => self.n.saturating_add(1),
                    Control::Set => self.n,
                };
                *mgr |= self.edit.set_string(n.to_string());
                self.n = n;
                n.into()
            }
        }
    };

    let driver = MyDriver {
        radio_group: UpdateHandle::new(),
    };
    let data = MyData::new(3);
    type MyList = ListView<kas::dir::Down, MyData, MyDriver>;

    let window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            #[layout(column)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget(handler = set_len)] controls -> usize = controls,
                #[widget] _ = Label::new("Contents of selected entry:"),
                #[widget] display: StringLabel = Label::from("Entry #0"),
                #[widget] _ = Separator::new(),
                #[widget(handler = set_radio)] list: ScrollBars<MyList> =
                    ScrollBars::new(ListView::new_with_view(driver, data)).with_bars(false, true),
            }
            impl {
                fn set_len(&mut self, mgr: &mut Manager, len: usize) -> Response<VoidMsg> {
                    let (opt_text, handle) = self.list.data_mut().set_len(len);
                    if let Some(text) = opt_text {
                        *mgr |= self.display.set_string(text);
                    }
                    mgr.trigger_update(handle, 0);
                    Response::None
                }
                fn set_radio(&mut self, mgr: &mut Manager, msg: ChildMsg<usize, EntryMsg>) -> Response<VoidMsg> {
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
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
