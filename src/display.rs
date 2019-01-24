// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Display widgets show information but are not interactive
// TODO: except `Entry`?

use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Core, CoreData, HasText, Editable, TkWidget};

/// A simple, static text label
#[widget(class = Class::Text(self))]
#[handler(response = NoResponse)]
#[derive(Clone, Default, Debug, Widget)]
pub struct Text {
    #[core] core: CoreData,
    text: String,
}

impl Text {
    /// Construct a new, empty instance
    pub fn new() -> Self {
        Text {
            core: Default::default(),
            text: String::new()
        }
    }
}

impl<T> From<T> for Text where String: From<T> {
    fn from(text: T) -> Self {
        Text {
            core: Default::default(),
            text: String::from(text)
        }
    }
}

impl HasText for Text {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
        self.text = text.into();
    }
}


/// An editable, single-line text box.
#[widget(class = Class::Entry(self))]
#[derive(Clone, Default, Widget)]
pub struct Entry<H: 'static> {
    #[core] core: CoreData,
    editable: bool,
    text: String,
    handler: H,
}

impl<H> Debug for Entry<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Entry {{ core: {:?}, editable: {:?}, text: {:?}, handler: <omitted> }}",
            self.core, self.editable, self.text)
    }
}

impl<R, H: Fn() -> R> Entry<H> {
    /// Construct an `Entry` with the given initial `text`.
    /// 
    /// The `handler` is called when [`Action::Activate`] is received
    /// (when the "enter" key is pressed)
    /// and its result is returned from the event handler.
    /// 
    /// [`Action::Activate`]: kas::event::Action::Activate
    pub fn new_on_activate(text: String, handler: H) -> Self {
        Entry {
            core: Default::default(),
            editable: true,
            text,
            handler,
        }
    }
}

impl Entry<()> {
    /// Construct an `Entry` which is optionally `editable`, and has the given
    /// inital `text`.
    pub fn new(editable: bool, text: String) -> Self {
        Entry {
            core: Default::default(),
            editable,
            text,
            handler: (),
        }
    }
}

impl<H> HasText for Entry<H> {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
        self.text = text.into();
    }
}

impl<H> Editable for Entry<H> {
    fn is_editable(&self) -> bool {
        self.editable
    }
}

impl Handler for Entry<()> {
    type Response = NoResponse;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Activate => {
                NoResponse
            }
            a @ _ => err_unhandled(a)
        }
    }
}

impl<R: From<NoResponse>, H: Fn() -> R> Handler for Entry<H> {
    type Response = R;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Activate => {
                (self.handler)()
            }
            a @ _ => err_unhandled(a)
        }
    }
}
