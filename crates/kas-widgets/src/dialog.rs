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

use crate::{Button, EditBox, Filler, Label, adapt::AdaptWidgetAny};
use kas::event::NamedKey;
use kas::prelude::*;
use kas::runner::AppData;
use kas::text::format::FormattableText;

#[derive(Copy, Clone, Debug)]
struct MessageBoxOk;

#[impl_self]
mod MessageBox {
    /// A simple message box.
    #[widget]
    #[layout(column! [self.label, self.button])]
    pub struct MessageBox<T: FormattableText + 'static> {
        core: widget_core!(),
        #[widget]
        label: Label<T>,
        #[widget]
        button: Button<Label<&'static str>>,
    }

    impl Self {
        /// Construct
        pub fn new(message: T) -> Self {
            MessageBox {
                core: Default::default(),
                label: Label::new(message),
                button: Button::new_msg(Label::new("Ok"), MessageBoxOk)
                    .with_access_key(NamedKey::Enter.into()),
            }
        }

        /// Build a [`Window`]
        pub fn into_window<A: AppData>(self, title: impl ToString) -> Window<A> {
            Window::new(self.map_any(), title).with_restrictions(true, true)
        }

        /// Display as a modal window with the given `title`
        pub fn display(self, cx: &mut EventCx, title: impl ToString) {
            cx.add_dataless_window(self.into_window(title));
        }
    }

    // TODO: call register_nav_fallback and close on Command::Escape, Enter
    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(Command::Escape, _) | Event::Command(Command::Enter, _) => {
                    cx.window_action(Action::CLOSE);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(MessageBoxOk) = cx.try_pop() {
                cx.action(self, Action::CLOSE);
            }
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

#[impl_self]
mod TextEdit {
    #[widget]
    #[layout(grid! {
        (0..3, 0) => self.edit,
        (0, 1) => Filler::maximize(),
        (1, 1) => Button::label_msg("&Cancel", MsgClose(false)),
        (2, 1) => Button::label_msg("&Save", MsgClose(true)),
    })]
    /// An editor over a `String`
    ///
    /// Emits a [`TextEditResult`] message when the "Ok" or "Cancel" button is
    /// pressed. When used as a pop-up, it is up to the caller to close on this
    /// message.
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
        pub fn set_text(&mut self, cx: &mut EventState, text: impl ToString) {
            self.edit.set_string(cx, text.to_string());
        }

        /// Build a [`Window`]
        pub fn into_window<A: AppData>(self, title: impl ToString) -> Window<A> {
            Window::new(self.map_any(), title)
        }

        fn close(&mut self, cx: &mut EventCx, commit: bool) -> IsUsed {
            cx.push(if commit {
                TextEditResult::Ok(self.edit.clone_string())
            } else {
                TextEditResult::Cancel
            });
            Used
        }
    }

    impl Events for Self {
        type Data = ();

        /* NOTE: this makes sense for a window but not an embedded editor.
        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());

            // Focus first item initially:
            if cx.nav_focus().is_none() {
                cx.next_nav_focus(self.id(), false, FocusSource::Synthetic);
            }
        }*/

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(Command::Escape, _) => self.close(cx, false),
                Event::Command(Command::Enter, _) => self.close(cx, true),
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(MsgClose(commit)) = cx.try_pop() {
                let _ = self.close(cx, commit);
            }
        }
    }
}
