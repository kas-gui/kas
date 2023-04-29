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
use kas::widget::*;

#[derive(Clone, Debug)]
enum Control {
    SetLen(usize),
    Reverse,
    Select(usize),
    Update(usize, String),
}

#[derive(Clone, Debug)]
enum Button {
    Set,
    Decr,
    Incr,
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
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

    let controls = singleton! {
        #[widget{
            layout = row! [
                "Number of rows:",
                self.edit,
                TextButton::new_msg("Set", Button::Set),
                TextButton::new_msg("−", Button::Decr),
                TextButton::new_msg("+", Button::Incr),
                TextButton::new_msg("↓↑", Control::Reverse),
            ];
        }]
        struct {
            core: widget_core!(),
            #[widget] edit: EditBox<(), impl EditGuard<()>> = EditBox::new("3")
                .on_afl(|cx, text| match text.parse::<usize>() {
                    Ok(n) => cx.push(n),
                    Err(_) => (),
                }),
            n: usize = 3,
        }
        impl Events for Self {
            type Data = ();

            fn handle_message(&mut self, _: &Self::Data, cx: &mut EventMgr) {
                if cx.last_child() == Some(widget_index![self.edit]) {
                    if let Some(n) = cx.try_pop::<usize>() {
                        if n != self.n {
                            self.n = n;
                            cx.push(Control::SetLen(n));
                        }
                    }
                } else if let Some(msg) = cx.try_pop::<Button>() {
                    let n = match msg {
                        Button::Decr => self.n.saturating_sub(1),
                        Button::Incr => self.n.saturating_add(1),
                        Button::Set => self.n,
                    };
                    *cx |= self.edit.set_string(n.to_string());
                    self.n = n;
                    cx.push(Control::SetLen(n));
                }
            }
        }
    };

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
            #[widget] controls: impl Widget<Data = ()> = controls,
            #[widget] display: StringLabel = Label::from("Entry #1"),
            #[widget(&self.active)] list: ScrollBarRegion<List<Direction, ListEntry>> =
                ScrollBarRegion::new(list).with_fixed_bars(false, true),
            active: usize = 0,
        }
        impl Events for Self {
            type Data = ();

            fn handle_message(&mut self, _: &(), cx: &mut EventMgr) {
                if let Some(control) = cx.try_pop() {
                    match control {
                        Control::SetLen(len) => {
                            cx.config_mgr(|mgr| {
                                self.list.inner_mut()
                                    .resize_with(&self.active, mgr, len, |n| ListEntry::new(n))
                            });
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
