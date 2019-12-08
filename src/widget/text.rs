// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};

use crate::class::{Align, Class, Editable, HasText};
use crate::event::{self, Action, EmptyMsg, Handler};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle, TextClass, TextProperties};
use crate::{CoreData, TkWindow, Widget, WidgetCore};

/// A simple text label
#[widget(class = Class::None)]
#[handler]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[core]
    core: CoreData,
    text: String,
}

impl Widget for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        size_handle.text_bound(&self.text, TextClass::Label, true, axis)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &event::Manager) {
        let props = TextProperties {
            class: TextClass::Label,
            multi_line: true,
            horiz: Align::Begin,
            vert: Align::Center,
        };
        draw_handle.text(self.core.rect, &self.text, props);
    }
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: ToString>(text: T) -> Self {
        Label {
            core: Default::default(),
            text: text.to_string(),
        }
    }
}

impl<T> From<T> for Label
where
    String: From<T>,
{
    fn from(text: T) -> Self {
        Label {
            core: Default::default(),
            text: String::from(text),
        }
    }
}

impl HasText for Label {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.text = text;
        tk.redraw(self.id());
    }
}

#[derive(Clone, Debug, PartialEq)]
enum LastEdit {
    None,
    Insert,
    Backspace,
    Clear,
    Paste,
}

impl Default for LastEdit {
    fn default() -> Self {
        LastEdit::None
    }
}

/// An editable, single-line text box.
#[widget(class = Class::Entry(self))]
#[derive(Clone, Default, Widget)]
pub struct Entry<H: 'static> {
    #[core]
    core: CoreData,
    editable: bool,
    text: String,
    old_state: Option<String>,
    last_edit: LastEdit,
    on_activate: H,
}

impl<H> Debug for Entry<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Entry {{ core: {:?}, editable: {:?}, text: {:?}, ... }}",
            self.core, self.editable, self.text
        )
    }
}

impl<H: 'static> Widget for Entry<H> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        size_handle.size_rules(self, axis)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &event::Manager) {
        draw_handle.draw(ev_mgr, self)
    }
}

impl Entry<()> {
    /// Construct an `Entry` with the given inital `text`.
    pub fn new<S: Into<String>>(text: S) -> Self {
        Entry {
            core: Default::default(),
            editable: true,
            text: text.into(),
            old_state: None,
            last_edit: LastEdit::None,
            on_activate: (),
        }
    }

    /// Set the event handler to be called on activation.
    ///
    /// The closure `f` is called when the `Entry` is activated (when the
    /// "enter" key is pressed). Its result is returned from the event handler.
    ///
    /// Technically, this consumes `self` and reconstructs another `Entry`
    /// with a different parameterisation.
    pub fn on_activate<R, H: Fn(&str) -> R>(self, f: H) -> Entry<H> {
        Entry {
            core: self.core,
            editable: self.editable,
            text: self.text,
            old_state: self.old_state,
            last_edit: self.last_edit,
            on_activate: f,
        }
    }
}

impl<H> Entry<H> {
    /// Set whether this `Entry` is editable.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    fn received_char(&mut self, tk: &mut dyn TkWindow, c: char) -> bool {
        if !self.editable {
            return false;
        }

        // TODO: Text selection and editing (see Unicode std. section 5.11)
        // Note that it may make sense to implement text shaping first.
        // For now we just filter control characters and append the rest.
        if c < '\u{20}' || (c >= '\u{7f}' && c <= '\u{9f}') {
            match c {
                '\u{03}' /* copy */ => {
                    // we don't yet have selection support, so just copy everything
                    tk.set_clipboard(self.text.clone());
                }
                '\u{08}' /* backspace */  => {
                    if self.last_edit != LastEdit::Backspace {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Backspace;
                    }
                    self.text.pop();
                }
                '\u{09}' /* tab */ => (),
                '\u{0A}' /* line feed */ => (),
                '\u{0B}' /* vertical tab */ => (),
                '\u{0C}' /* form feed */ => (),
                '\u{0D}' /* carriage return (\r) */ => return true,
                '\u{16}' /* paste */ => {
                    if self.last_edit != LastEdit::Paste {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Paste;
                    }
                    if let Some(content) = tk.get_clipboard() {
                        // We cut the content short on control characters and
                        // ignore them (preventing line-breaks and ignoring any
                        // actions such as recursive-paste).
                        let mut end = content.len();
                        for (i, b) in content.as_bytes().iter().cloned().enumerate() {
                            if b < 0x20 || (b >= 0x7f && b <= 0x9f) {
                                end = i;
                                break;
                            }
                        }
                        self.text.push_str(&content[0..end]);
                    }
                }
                '\u{1A}' /* undo and redo */ => {
                    // TODO: maintain full edit history (externally?)
                    // NOTE: undo *and* redo shortcuts map to this control char
                    if let Some(state) = self.old_state.as_mut() {
                        std::mem::swap(state, &mut self.text);
                        self.last_edit = LastEdit::None;
                    }
                }
                '\u{1B}' /* escape */ => (),
                '\u{7f}' /* delete */ => {
                    if self.last_edit != LastEdit::Clear {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Clear;
                    }
                    self.text.clear();
                }
                _ => (),
            };
        } else {
            if self.last_edit != LastEdit::Insert {
                self.old_state = Some(self.text.clone());
                self.last_edit = LastEdit::Insert;
            }
            self.text.push(c);
        }
        tk.redraw(self.id());
        false
    }
}

impl<H> HasText for Entry<H> {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.text = text;
        tk.redraw(self.id());
    }
}

impl<H> Editable for Entry<H> {
    fn is_editable(&self) -> bool {
        self.editable
    }

    fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }
}

impl Handler for Entry<()> {
    type Msg = EmptyMsg;

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> EmptyMsg {
        match action {
            Action::Activate => tk.update_data(&mut |data| data.set_char_focus(self.id())),
            Action::ReceivedCharacter(c) => {
                self.received_char(tk, c);
            }
        }
        EmptyMsg
    }
}

impl<M: From<EmptyMsg>, H: Fn(&str) -> M> Handler for Entry<H> {
    type Msg = M;

    fn handle_action(&mut self, tk: &mut dyn TkWindow, action: Action) -> M {
        match action {
            Action::Activate => {
                tk.update_data(&mut |data| data.set_char_focus(self.id()));
                EmptyMsg.into()
            }
            Action::ReceivedCharacter(c) => {
                if self.received_char(tk, c) {
                    ((self.on_activate)(&self.text)).into()
                } else {
                    EmptyMsg.into()
                }
            }
        }
    }
}
