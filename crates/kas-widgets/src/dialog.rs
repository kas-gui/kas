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

use crate::{adapt::AdaptWidgetAny, EditBox, Filler, Label, TextButton};
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
            Window::new(self.map_any(), title)
                .with_restrictions(true, true)
        }

    }

    impl Events for Self {
        type Data = ();

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(MessageBoxOk) = cx.try_pop() {
                cx.send_action(Action::CLOSE);
            }
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.enable_alt_bypass(self.id_ref(), true);
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

        /// Set text
        pub fn set_text(&mut self, text: impl ToString) -> Action {
            self.edit.set_string(text.to_string())
        }

        /// Build a [`Window`]
        pub fn into_window<A: 'static>(self, title: impl ToString) -> Window<A> {
            Window::new(self.map_any(), title)
        }

        fn close(&mut self, cx: &mut EventCx, commit: bool) -> Response {
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

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());

            // Focus first item initially:
            if cx.nav_focus().is_none() {
                cx.next_nav_focus(self.id(), false, true);
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> Response {
            match event {
                Event::Command(Command::Escape) => self.close(cx, false),
                Event::Command(Command::Enter) => self.close(cx, true),
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(MsgClose(commit)) = cx.try_pop() {
                let _ = self.close(cx, commit);
            }
        }
    }
}
