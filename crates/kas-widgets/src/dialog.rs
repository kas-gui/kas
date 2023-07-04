// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dialog boxes
//!
//! KAS dialog boxes are pre-configured windows, usually allowing some
//! customisation.
//!
//! # Design status
//!
//! At the current time, only a minimal selection of dialog boxes are provided
//! and their design is likely to change.

use crate::{EditBox, Filler, Label, TextButton};
use kas::event::{Command, VirtualKeyCode};
use kas::model::{SharedRc, SingleDataMut};
use kas::prelude::*;
use kas::text::format::FormattableText;

#[derive(Copy, Clone, Debug)]
struct MessageBoxOk;

impl_scope! {
    /// A simple message box.
    #[widget{
        layout = column! [self.label, self.button];
    }]
    pub struct MessageBox<T: FormattableText + 'static> {
        core: widget_core!(),
        #[widget]
        label: Label<T>,
        #[widget]
        button: TextButton,
    }

    impl Self {
        /// Construct
        pub fn new(message: T) -> Self {
            MessageBox {
                core: Default::default(),
                label: Label::new(message),
                button: TextButton::new_msg("Ok", MessageBoxOk).with_keys(&[
                    VirtualKeyCode::Return,
                    VirtualKeyCode::Space,
                    VirtualKeyCode::NumpadEnter,
                ]),
            }
        }

        /// Build a [`Window`]
        pub fn into_window(self, title: impl ToString) -> Window {
            Window::new(self, title)
                .with_restrictions(true, true)
        }

    }

    impl Events for Self {
        type Data = ();

        fn handle_message(&mut self, mgr: &mut EventMgr) {
            if let Some(MessageBoxOk) = mgr.try_pop() {
                mgr.send_action(Action::CLOSE);
            }
        }

        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.enable_alt_bypass(self.id_ref(), true);
        }
    }
}

#[derive(Clone, Debug)]
struct MsgClose(bool);

impl_scope! {
    #[widget{
        layout = grid! {
            (0..3, 0) => self.edit,
            (0, 1) => Filler::maximize(),
            (1, 1) => TextButton::new_msg("&Cancel", MsgClose(false)),
            (2, 1) => TextButton::new_msg("&Save", MsgClose(true)),
        };
    }]
    /// An editor over a shared `String`
    ///
    /// The shared data is updated only when the "Save" button is pressed.
    pub struct TextEdit<T: SingleDataMut<Item = String> + 'static = SharedRc<String>> {
        core: widget_core!(),
        data: T,
        #[widget]
        edit: EditBox,
    }

    impl Self {
        /// Construct
        pub fn new(multi_line: bool, data: T) -> Self {
            let text = data.get_cloned(&()).unwrap();
            TextEdit {
                core: Default::default(),
                data,
                edit: EditBox::new(text).with_multi_line(multi_line),
            }
        }

        /// Build a [`Window`]
        pub fn into_window(self, title: impl ToString) -> Window {
            Window::new(self, title)
        }

        fn close(&mut self, mgr: &mut EventMgr, commit: bool) -> Response {
            if commit {
                self.data.set(mgr, &(), self.edit.get_string());
            }
            mgr.send_action(Action::CLOSE);
            Response::Used
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.register_nav_fallback(self.id());

            // Focus first item initially:
            if mgr.nav_focus().is_none() {
                mgr.next_nav_focus(self.id(), false, true);
            }
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape) => self.close(mgr, false),
                Event::Command(Command::Enter) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr) {
            if let Some(MsgClose(commit)) = mgr.try_pop() {
                let _ = self.close(mgr, commit);
            }
        }
    }
}
