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
use kas::widgets::*;

thread_local! {
    pub static RADIO: RadioBoxGroup = Default::default();
}

#[derive(Clone, Debug)]
enum Control {
    Set(usize),
    Dir,
}

#[derive(Clone, Debug)]
enum Button {
    Set,
    Decr,
    Incr,
}

#[derive(Clone, Debug)]
enum EntryMsg {
    Select(usize),
    Update(usize, String),
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
#[derive(Clone, Debug)]
struct ListEntryGuard(usize);
impl EditGuard for ListEntryGuard {
    fn edit(entry: &mut EditField<Self>, mgr: &mut EventMgr) {
        mgr.push_msg(EntryMsg::Update(entry.guard.0, entry.get_string()));
    }
}

impl_scope! {
    // The list entry
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: [
            row: [self.label, self.radio],
            self.entry,
        ];
    }]
    struct ListEntry {
        core: widget_core!(),
        #[widget]
        label: StringLabel,
        #[widget]
        radio: RadioBox,
        #[widget]
        entry: EditBox<ListEntryGuard>,
    }
}

impl ListEntry {
    fn new(n: usize, active: bool) -> Self {
        // Note: we embed `n` into messages here. A possible alternative: use
        // List::on_message to pop the message and push `(usize, EntryMsg)`.
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioBox::new("display this entry", RADIO.with(|g| g.clone()))
                .with_state(active)
                .on_select(move |mgr| mgr.push_msg(EntryMsg::Select(n))),
            entry: EditBox::new(format!("Entry #{}", n + 1)).with_guard(ListEntryGuard(n)),
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = impl_singleton! {
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
            #[widget] edit: impl HasString = EditBox::new("3")
                .on_afl(|text, mgr| match text.parse::<usize>() {
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
                            mgr.push_msg(Control::Set(n));
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

    let entries = vec![
        ListEntry::new(0, true),
        ListEntry::new(1, false),
        ListEntry::new(2, false),
    ];
    let list = List::new_dir_vec(Direction::Down, entries);

    let window = impl_singleton! {
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
            #[widget] list: ScrollBarRegion<List<Direction, ListEntry>> =
                ScrollBarRegion::new(list).with_fixed_bars(false, true),
            active: usize = 0,
        }
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(control) = mgr.try_pop_msg() {
                    match control {
                        Control::Set(len) => {
                            let active = self.active;
                            mgr.set_rect_mgr(|mgr| {
                                self.list.inner_mut()
                                    .resize_with(mgr, len, |n| ListEntry::new(n, n == active))
                            });
                        }
                        Control::Dir => {
                            let dir = self.list.direction().reversed();
                            *mgr |= self.list.set_direction(dir);
                        }
                    }
                } else if let Some(msg) = mgr.try_pop_msg() {
                    match msg {
                        EntryMsg::Select(n) => {
                            self.active = n;
                            let text = self.list[n].entry.get_string();
                            *mgr |= self.display.set_string(text);
                        }
                        EntryMsg::Update(n, text) => {
                            if n == self.active {
                                *mgr |= self.display.set_string(text);
                            }
                        }
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Dynamic widget demo" }
        }
    };

    let theme = kas::theme::ShadedTheme::new();
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
}
