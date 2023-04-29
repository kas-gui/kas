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
//! Conclusion: naive lists are perfectly fine for 100 entries; even with 10k
//! entries in a debug build only initialisation (and to a lesser extent
//! resizing) is slow.
//! In a release build, 250k entries (1M widgets) is quite viable!

use kas::prelude::*;
use kas::row;
use kas::widget::adapter::WithAny;
use kas::widget::edit::{EditBox, EditField, EditGuard};
use kas::widget::{Label, List, RadioButton, ScrollBarRegion, Separator, StringLabel, TextButton};

#[derive(Clone, Debug)]
enum Control {
    None,
    SetLen(usize),
    DecrLen,
    IncrLen,
    Reverse,
    Select(usize),
    Update(usize, String),
}

#[derive(Clone, Debug)]
struct ListEntryGuard(usize);
impl EditGuard<()> for ListEntryGuard {
    fn activate(edit: &mut EditField<(), Self>, _: &(), mgr: &mut EventMgr) -> Response {
        mgr.push(Control::Select(edit.guard.0));
        Response::Used
    }

    fn edit(edit: &mut EditField<(), Self>, _: &(), mgr: &mut EventMgr) {
        mgr.push(Control::Update(edit.guard.0, edit.get_string()));
    }
}

impl_scope! {
    // The list entry
    #[widget{
        Data = usize;
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
        radio: RadioButton<usize>,
        // We deliberately use these widgets to store state instead of passing.
        // See examples/data-list-view.rs for a better option.
        #[widget(&())]
        edit: EditBox<(), ListEntryGuard>,
    }
}

impl ListEntry {
    fn new(n: usize) -> Self {
        // Note: we embed `n` into messages here. A possible alternative: use
        // List::on_message to pop the message and push `(usize, Control)`.
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioButton::new_msg(
                "display this entry",
                move |active| *active == n,
                move || Control::Select(n),
            ),
            edit: EditBox::new(format!("Entry #{}", n + 1)).with_guard(ListEntryGuard(n)),
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
    let list = List::new_dir_vec(Direction::Down, entries);

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
            // FIXME: pass &self.list.len() here (requires removal of Widget::get_child):
            #[widget(&3)] controls: impl Widget<Data = usize> = controls,
            #[widget] display: StringLabel = Label::from("Entry #1"),
            #[widget(&self.active)] list: ScrollBarRegion<List<Direction, ListEntry>> =
                ScrollBarRegion::new(list).with_fixed_bars(false, true),
            active: usize = 0,
        }
        impl Events for Self {
            type Data = ();

            fn handle_message(&mut self, _: &(), cx: &mut EventMgr) {
                let mut new_len = None;

                if let Some(control) = cx.try_pop() {
                    match control {
                        Control::None => (),
                        Control::SetLen(len) => {
                            new_len = Some(len);
                        }
                        Control::DecrLen => {
                            new_len = self.list.len().checked_sub(1);
                        }
                        Control::IncrLen => {
                            new_len = self.list.len().checked_add(1);
                        }
                        Control::Reverse => {
                            let dir = self.list.direction().reversed();
                            *cx |= self.list.set_direction(dir);
                        }
                        Control::Select(n) => {
                            self.active = n;
                            let entry = &mut self.list[n];
                            let text = entry.edit.get_string();
                            *cx |= self.display.set_string(text);
                            cx.update(self.as_node_mut(&()));
                        }
                        Control::Update(n, text) => {
                            if n == self.active {
                                *cx |= self.display.set_string(text);
                            }
                        }
                    }

                    if let Some(len) = new_len {
                        cx.config_mgr(|mgr| {
                            self.list.inner_mut()
                                .resize_with(&self.active, mgr, len, |n| ListEntry::new(n))
                        });
                    }
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
