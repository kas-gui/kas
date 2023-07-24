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

use crate::adapter::WithAny;
use crate::{EditBox, Filler, Label, TextButton};
use kas::event::{Command, VirtualKeyCode};
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
        pub fn into_window<A: 'static>(self, title: impl ToString) -> Window<A> {
            Window::new(WithAny::new(self), title)
                .with_restrictions(true, true)
        }

    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            if let Some(MessageBoxOk) = mgr.try_pop() {
                mgr.send_action(Action::CLOSE);
            }
        }

        fn configure(&mut self, mgr: &mut ConfigMgr) {
            mgr.enable_alt_bypass(self.id_ref(), true);
        }
    }
}

/// Message sent by [`TextEdit`] on closure.
#[derive(Debug)]
pub enum TextEditResult {
    Cancel,
    Ok(String),
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
    /// An editor over a `String`
    ///
    /// Emits a [`TextEditResult`] message on closure.
    pub struct TextEdit {
        core: widget_core!(),
        #[widget]
        edit: EditBox,
    }

    impl Self {
        /// Construct
        pub fn new(text: impl ToString, multi_line: bool) -> Self {
            TextEdit {
                core: Default::default(),
                edit: EditBox::text(text).with_multi_line(multi_line),
            }
        }

        /// Build a [`Window`]
        pub fn into_window<A: 'static>(self, title: impl ToString) -> Window<A> {
            Window::new(WithAny::new(self), title)
        }

        fn close(&mut self, cx: &mut EventMgr, commit: bool) -> Response {
            cx.push(if commit {
                TextEditResult::Ok(self.edit.get_string())
            } else {
                TextEditResult::Cancel
            });
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

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape) => self.close(mgr, false),
                Event::Command(Command::Enter) => self.close(mgr, true),
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            if let Some(MsgClose(commit)) = mgr.try_pop() {
                let _ = self.close(mgr, commit);
            }
        }
    }
}
