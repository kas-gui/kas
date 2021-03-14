// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widget example
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
use kas::widget::*;

thread_local! {
    pub static RADIO: UpdateHandle = UpdateHandle::new();
}

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
//
// Use of a compound listing here with five child widgets (RadioBox is a
// compound widget) slows down list resizing significantly (more so in debug
// builds).
//
// Use of an embedded RadioBox demonstrates another performance issue:
// activating any RadioBox sends a message to all others using the same
// UpdateHandle, which is quite slow with thousands of entries!
// (This issue does not occur when RadioBoxes are independent.)
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

impl ListEntry {
    fn new(n: usize, active: bool) -> Self {
        ListEntry {
            core: Default::default(),
            layout_data: Default::default(),
            label: Label::new(format!("Entry number {}", n + 1)),
            radio: RadioBox::new("display this entry", RADIO.with(|h| *h))
                .with_state(active)
                .on_select(move |_| Some(EntryMsg::Select)),
            entry: EditBox::new(format!("Entry #{}", n + 1)).with_guard(ListEntryGuard),
        }
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

    let entries = vec![
        ListEntry::new(0, true),
        ListEntry::new(1, false),
        ListEntry::new(2, false),
    ];

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
                #[widget(handler = set_radio)] list: ScrollBarRegion<Column<ListEntry>> =
                    ScrollBarRegion::new2(Column::new(entries)).with_bars(false, true),
                #[widget] _ = Filler::maximize(),
                active: usize = 0,
            }
            impl {
                fn set_len(&mut self, mgr: &mut Manager, len: usize) -> Response<VoidMsg> {
                    let active = self.active;
                    let old_len = self.list.len();
                    *mgr |= self.list.inner_mut().resize_with(len, |n| ListEntry::new(n, n == active));
                    if active >= old_len && active < len {
                        let _ = self.set_radio(mgr, (active, EntryMsg::Select));
                    }
                    Response::None
                }
                fn set_radio(&mut self, mgr: &mut Manager, msg: (usize, EntryMsg)) -> Response<VoidMsg> {
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
                    Response::None
                }
            }
        },
    );

    let theme = kas_theme::ShadedTheme::new();
    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
