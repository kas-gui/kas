// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};

use crate::class::{Class, Editable, HasText};
use crate::event::{Action, EmptyMsg, Handler};
use crate::macros::Widget;
use crate::{Core, CoreData, TkWindow};

/// A simple text label
#[widget(class = Class::Label(self), layout = derive)]
#[handler]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[core]
    core: CoreData,
    text: String,
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
        tk.redraw(self);
    }
}

/// An editable, single-line text box.
#[widget(class = Class::Entry(self), layout = derive)]
#[derive(Clone, Default, Widget)]
pub struct Entry<H: 'static> {
    #[core]
    core: CoreData,
    editable: bool,
    text: String,
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

impl Entry<()> {
    /// Construct an `Entry` with the given inital `text`.
    pub fn new<S: Into<String>>(text: S) -> Self {
        Entry {
            core: Default::default(),
            editable: true,
            text: text.into(),
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
        // TODO: allow edit position other than end
        match c {
            '\u{8}' /* backspace */  => { self.text.pop(); }
            '\u{7f}' /* delete */ => self.text.clear(),
            '\r' /* enter */ => return true,
            _ => self.text.push(c),
        };
        tk.redraw(self);
        false
    }
}

impl<H> HasText for Entry<H> {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String) {
        self.text = text;
        tk.redraw(self);
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
            Action::Activate => tk.update_data(&mut |data| data.set_grab(self.id())),
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
                tk.update_data(&mut |data| data.set_grab(self.id()));
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
