// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, Response, err_num, err_unhandled};
use crate::{Class, Core, CoreData, HasText, Editable, TkWidget};

/// A simple text label
#[widget(class = Class::Label(self))]
#[handler(msg = ())]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[core] core: CoreData,
    text: String,
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: ToString>(text: T) -> Self {
        Label {
            core: Default::default(),
            text: text.to_string()
        }
    }
}

impl<T> From<T> for Label where String: From<T> {
    fn from(text: T) -> Self {
        Label {
            core: Default::default(),
            text: String::from(text)
        }
    }
}

impl HasText for Label {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_string(&mut self, tk: &mut dyn TkWidget, text: String) {
        tk.set_text(self.tkd(), &text);
        self.text = text;
    }
}


/// An editable, single-line text box.
#[widget(class = Class::Entry(self))]
#[derive(Clone, Default, Widget)]
pub struct Entry<H: 'static> {
    #[core] core: CoreData,
    editable: bool,
    text: String,
    on_activate: H,
}

impl<H> Debug for Entry<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Entry {{ core: {:?}, editable: {:?}, text: {:?}, ... }}",
            self.core, self.editable, self.text)
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
    /// The closure `f` is called when [`Action::Activate`] is received (when the
    /// "enter" key is pressed). Its result is returned from the event handler.
    /// 
    /// Technically, this consumes `self` and reconstructs another `Entry`
    /// with a different parameterisation.
    /// 
    /// [`Action::Activate`]: kas::event::Action::Activate
    pub fn on_activate<M, H: Fn() -> M>(self, f: H) -> Entry<H> {
        Entry {
            core: self.core,
            editable: self.editable,
            text: self.text,
            on_activate: f
        }
    }
}

impl<H> Entry<H> {
    /// Set whether this `Entry` is editable.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
}

impl<H> HasText for Entry<H> {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_string(&mut self, tk: &mut dyn TkWidget, text: String) {
        tk.set_text(self.tkd(), &text);
        self.text = text;
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
    type Msg = ();
    
    fn handle(&mut self, _tk: &mut dyn TkWidget, action: Action, num: u32)
        -> Response<Self::Msg>
    {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Activate => Response::None,
            a @ _ => err_unhandled(a)
        }
    }
}

impl<M, H: Fn() -> M> Handler for Entry<H> {
    type Msg = M;
    
    fn handle(&mut self, _tk: &mut dyn TkWidget, action: Action, num: u32)
        -> Response<Self::Msg>
    {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Activate => {
                ((self.on_activate)()).into()
            }
            a @ _ => err_unhandled(a)
        }
    }
}
