// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::any::TypeId;
use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Core, CoreData, HasBool, HasText, TkWidget};

/// A checkable box with optional label
#[widget(class = Class::CheckBox(self))]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<OT: 'static> {
    #[core] core: CoreData,
    label: String,
    state: bool,
    on_toggle: OT
}

impl<H> Debug for CheckBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CheckBox {{ core: {:?}, state: {:?}, label: {:?}, ... }}",
            self.core, self.state, self.label)
    }
}

impl<R, OT: Fn(bool) -> R> CheckBox<OT> {
    /// Construct a checkbox with a given `label` which calls `f` when toggled.
    /// 
    /// This is a shortcut for `CheckBox::new(label).on_toggle(f)`.
    /// 
    /// Checkbox labels are optional; if no label is desired, use an empty
    /// string.
    /// 
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    pub fn new_on<S: Into<String>>(label: S, f: OT) -> Self {
        CheckBox {
            core: Default::default(),
            label: label.into(),
            state: false,
            on_toggle: f
        }
    }
}

impl CheckBox<()> {
    /// Construct a checkbox with a given `label`.
    /// 
    /// CheckBox labels are optional; if no label is desired, use an empty
    /// string.
    pub fn new<S: Into<String>>(label: S) -> Self {
        CheckBox {
            core: Default::default(),
            label: label.into(),
            state: false, 
            on_toggle: ()
        }
    }
    
    /// Set the event handler to be called on toggle.
    /// 
    /// The closure `f` is called with the new state of the checkbox when
    /// toggled, and the result of `f` is returned from the event handler.
    pub fn on_toggle<R, OT: Fn() -> R>(self, f: OT) -> CheckBox<OT> {
        CheckBox {
            core: self.core,
            label: self.label,
            state: self.state,
            on_toggle: f
        }
    }
}

impl<OT: 'static> CheckBox<OT> {
    /// Set the initial state of the checkbox.
    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }
    
    /// Set the initial state of the checkbox.
    pub fn state(mut self, state: bool) -> Self {
        self.state = state;
        self
    }
    
    /// Whether this checkbox has a handler set on toggle actions.
    // TODO: this needs to be defined on a trait that the toolkit can access
    pub fn has_on_toggle(&self) -> bool {
        TypeId::of::<OT>() != TypeId::of::<()>()
    }
}

impl<H> HasBool for CheckBox<H> {
    fn get_bool(&self) -> bool {
        self.state
    }
    
    fn set_bool(&mut self, tk: &TkWidget, state: bool) {
        self.state = state;
        tk.set_bool(self.tkd(), state);
    }
}

impl<H> HasText for CheckBox<H> {
    fn get_text(&self) -> &str {
        &self.label
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_text(self.tkd(), text);
        self.label = text.into();
    }
}

impl Handler for CheckBox<()> {
    type Response = NoResponse;
    
    fn handle_action(&mut self, _: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Toggle => NoResponse,
            a @ _ => err_unhandled(a)
        }
    }
}

impl<R: From<NoResponse>, H: Fn(bool) -> R> Handler for CheckBox<H> {
    type Response = R;
    
    fn handle_action(&mut self, tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Toggle => {
                self.state = tk.get_bool(self.tkd());  // sync
                (self.on_toggle)(self.state)
            }
            a @ _ => err_unhandled(a)
        }
    }
}


/// A push-button with a text label
// TODO: abstract out text part?
#[widget(class = Class::Button(self))]
#[derive(Clone, Default, Widget)]
pub struct TextButton<H: 'static> {
    #[core] core: CoreData,
    label: String,
    on_click: H
}

impl<H> Debug for TextButton<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextButton {{ core: {:?}, label: {:?}, ... }}",
            self.core, self.label)
    }
}

impl<R, H: Fn() -> R> TextButton<H> {
    /// Construct a button with a given `label` which calls `f` on click.
    /// 
    /// This is a shortcut for `TextButton::new(label).on_click(f)`.
    /// 
    /// The closure `f` is called when the button is pressed, and its result is
    /// returned from the event handler.
    pub fn new_on<S: Into<String>>(label: S, f: H) -> Self {
        TextButton {
            core: Default::default(),
            label: label.into(),
            on_click: f
        }
    }
}

impl TextButton<()> {
    /// Construct a button with a given `label`.
    pub fn new<S: Into<String>>(label: S) -> Self {
        TextButton {
            core: Default::default(),
            label: label.into(),
            on_click: ()
        }
    }
    
    /// Set the event handler to be called on click.
    /// 
    /// The closure `f` is called when the button is pressed, and its result is
    /// returned from the event handler.
    pub fn on_click<R, H: Fn() -> R>(self, f: H) -> TextButton<H> {
        TextButton {
            core: self.core,
            label: self.label,
            on_click: f
        }
    }
}

// impl<H> From<&'static str> for TextButton<NoResponse, H>
//     where H: Fn(()) -> NoResponse
// {
//     fn from(label: &'static str) -> Self {
//         TextButton::new(label, |()| NoResponse)
//     }
// }

impl<H> HasText for TextButton<H> {
    fn get_text(&self) -> &str {
        &self.label
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_text(self.tkd(), text);
        self.label = text.into();
    }
}


impl Handler for TextButton<()> {
    type Response = NoResponse;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Button => NoResponse,
            a @ _ => err_unhandled(a)
        }
    }
}

impl<R: From<NoResponse>, H: Fn() -> R> Handler for TextButton<H> {
    type Response = R;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Button => (self.on_click)(),
            a @ _ => err_unhandled(a)
        }
    }
}

/// TODO: for use with dialogs...
pub mod button {
    use super::TextButton;
    
    pub fn ok<R, H: Fn() -> R>(on_click: H) -> TextButton<H> {
        TextButton::new_on("Ok", on_click)
    }
}
