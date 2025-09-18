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

use crate::{AccessLabel, Button, EditBox, Filler, Label, adapt::AdaptWidgetAny};
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
        button: Button<AccessLabel>,
    }

    impl Self {
        /// Construct
        pub fn new(message: T) -> Self {
            MessageBox {
                core: Default::default(),
                label: Label::new(message),
                button: Button::label_msg("&Ok", MessageBoxOk),
            }
        }

        /// Build a [`Window`]
        pub fn into_window<A: AppData>(self, title: impl ToString) -> Window<A> {
            Window::new(self.map_any(), title).with_restrictions(true, true)
        }

        /// Display as a modal window with the given `title`
        pub fn display(self, cx: &mut EventCx, title: impl ToString) {
            cx.add_dataless_window(self.into_window(title), true);
        }
    }

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

#[derive(Copy, Clone, Debug)]
pub enum UnsavedResult {
    Save,
    Discard,
    Cancel,
}

#[impl_self]
mod AlertUnsaved {
    /// Alert user that they have unsaved changes
    #[widget]
    #[layout(column! [
        self.label,
        row![self.save, self.discard, self.cancel],
    ])]
    pub struct AlertUnsaved<T: FormattableText + 'static> {
        core: widget_core!(),
        parent: Id,
        title: String,
        #[widget]
        label: Label<T>,
        #[widget]
        save: Button<AccessLabel>,
        #[widget]
        discard: Button<AccessLabel>,
        #[widget]
        cancel: Button<AccessLabel>,
    }

    impl Self {
        /// Construct
        pub fn new(message: T) -> Self {
            AlertUnsaved {
                core: Default::default(),
                parent: Id::default(),
                title: "Unsaved changes".to_string(),
                label: Label::new(message),
                save: Button::label_msg("&Save", UnsavedResult::Save),
                discard: Button::label_msg("&Discard", UnsavedResult::Discard),
                cancel: Button::label_msg("&Cancel", UnsavedResult::Cancel),
            }
        }

        /// Set a custom window title
        pub fn with_title(mut self, title: impl ToString) -> Self {
            self.title = title.to_string();
            self
        }

        /// Display as a modal window
        ///
        /// On closure, an [`UnsavedResult`] message will be sent to `parent`.
        pub fn display_for(mut self, cx: &mut EventCx, parent: Id) {
            self.parent = parent;
            let title = std::mem::take(&mut self.title);
            let window = Window::new(self.map_any(), title).with_restrictions(true, true);
            cx.add_dataless_window(window, true);
        }
    }

    impl Events for Self {
        type Data = ();

        fn configure(&mut self, cx: &mut ConfigCx) {
            cx.register_nav_fallback(self.id());
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let result = match event {
                Event::Command(Command::Escape, _) => UnsavedResult::Cancel,
                Event::Command(Command::Enter, _) => {
                    if let Some(focus) = cx.nav_focus() {
                        if self.save.is_ancestor_of(focus) {
                            UnsavedResult::Save
                        } else if self.discard.is_ancestor_of(focus) {
                            UnsavedResult::Discard
                        } else if self.cancel.is_ancestor_of(focus) {
                            UnsavedResult::Cancel
                        } else {
                            return Unused;
                        }
                    } else {
                        return Unused;
                    }
                }
                _ => return Unused,
            };

            cx.send(self.parent.clone(), result);
            cx.window_action(Action::CLOSE);
            Used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(result) = cx.try_pop::<UnsavedResult>() {
                cx.send(self.parent.clone(), result);
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
