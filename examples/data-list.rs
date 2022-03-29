// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data list example (direct representation)
//!
//! This example exists in part to demonstrate use of dynamically-allocated
//! widgets (note also one can use `Column<Box<dyn Widget<Msg = ()>>>`).
//!
//! In part, this also serves as a stress-test of how many widgets it is viable
//! to have in an app. In my testing:
//!
//! -   hundreds of widgets performs mostly flawlessly even in debug mode
//! -   thousands of widgets performs flawlessly in release mode
//! -   hundreds of thousands of widgets has some issues (slow creation,
//!     very slow activation of a RadioBox in a chain hundreds-of-thousands
//!     long), but in many ways still performs well in release mode

use kas::prelude::*;
use kas::widgets::*;

thread_local! {
    pub static RADIO: RadioBoxGroup = Default::default();
}

#[derive(Clone, Debug, VoidMsg)]
enum Control {
    Set(usize),
    Dir,
}

#[derive(Clone, Debug, VoidMsg)]
enum Button {
    Set,
    Decr,
    Incr,
}

#[derive(Clone, Debug, VoidMsg)]
enum EntryMsg {
    Select,
    Update(String),
}

// TODO: it would be nicer to use EditBox::new(..).on_edit(..), but that produces
// an object with unnamable type, which is a problem.
#[derive(Clone, Debug)]
struct ListEntryGuard;
impl EditGuard for ListEntryGuard {
    type Msg = EntryMsg;

    fn edit(entry: &mut EditField<Self>, _: &mut EventMgr) -> Option<Self::Msg> {
        Some(EntryMsg::Update(entry.get_string()))
    }
}

widget! {
    // The list entry
    //
    // Use of a compound listing here with five child widgets (RadioBox is a
    // compound widget) slows down list resizing significantly (more so in debug
    // builds).
    //
    // Use of an embedded RadioBox demonstrates another performance issue:
    // activating any RadioBox sends a message to all others using the same
    // UpdateHandle, which is quite slow with thousands of entries!
    // (This issue does not occur when RadioBoxes are independent.)
    #[derive(Clone, Debug)]
    #[widget{
        layout = column: *;
        msg = EntryMsg;
    }]
    struct ListEntry {
        #[widget_core]
        core: CoreData,
        #[widget]
        label: StringLabel,
        #[widget]
        radio: RadioBox<EntryMsg>,
        #[widget]
        entry: EditBox<ListEntryGuard>,
    }
}

impl ListEntry {
    fn new(n: usize, active: bool) -> Self {
        ListEntry {
            core: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioBox::new("display this entry", RADIO.with(|g| g.clone()))
                .with_state(active)
                .on_select(move |_| Some(EntryMsg::Select)),
            entry: EditBox::new(format!("Entry #{}", n + 1)).with_guard(ListEntryGuard),
        }
    }
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let controls = make_widget! {
        #[widget{
            layout = row: *;
            msg = Control;
        }]
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
        impl Self {
            fn activate(&mut self, _: &mut EventMgr, n: usize) -> Control {
                self.n = n;
                Control::Set(n)
            }
            fn button(&mut self, mgr: &mut EventMgr, msg: Button) -> Control {
                let n = match msg {
                    Button::Set => self.n,
                    Button::Decr => self.n.saturating_sub(1),
                    Button::Incr => self.n.saturating_add(1),
                };
                *mgr |= self.edit.set_string(n.to_string());
                self.n = n;
                Control::Set(n)
            }
        }
    };

    let entries = vec![
        ListEntry::new(0, true),
        ListEntry::new(1, false),
        ListEntry::new(2, false),
    ];
    let list = IndexedList::new_with_direction(Direction::Down, entries);

    let window = Window::new(
        "Dynamic widget demo",
        make_widget! {
            #[widget{
                layout = column: *;
                msg = VoidMsg;
            }]
            struct {
                #[widget] _ = Label::new("Demonstration of dynamic widget creation / deletion"),
                #[widget(use_msg = control)] controls -> Control = controls,
                #[widget] _ = Label::new("Contents of selected entry:"),
                #[widget] display: StringLabel = Label::from("Entry #0"),
                #[widget] _ = Separator::new(),
                #[widget(use_msg = set_radio)] list: ScrollBarRegion<IndexedList<Direction, ListEntry>> =
                    ScrollBarRegion::new(list).with_bars(false, true),
                active: usize = 0,
            }
            impl Self {
                fn control(&mut self, mgr: &mut EventMgr, control: Control) {
                    match control {
                        Control::Set(len) => {
                            let active = self.active;
                            let old_len = self.list.len();
                            mgr.set_rect_mgr(|mgr| {
                                self.list.inner_mut()
                                    .resize_with(mgr, len, |n| ListEntry::new(n, n == active))
                            });
                            if active >= old_len && active < len {
                                let _ = self.set_radio(mgr, (active, EntryMsg::Select));
                            }
                        }
                        Control::Dir => {
                            let dir = self.list.direction().reversed();
                            *mgr |= self.list.set_direction(dir);
                        }
                    }
                }
                fn set_radio(&mut self, mgr: &mut EventMgr, msg: (usize, EntryMsg)) {
                    let n = msg.0;
                    match msg.1 {
                        EntryMsg::Select => {
                            self.active = n;
                            let text = self.list[n].entry.get_string();
                            *mgr |= self.display.set_string(text);
                        }
                        EntryMsg::Update(text) => {
                            if n == self.active {
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
