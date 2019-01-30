// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Core, CoreData, HasBool, HasText, TkWidget};

/// A checkable box with optional label
#[widget(class = Class::CheckBox(self))]
#[derive(Clone, Default, Widget)]
pub struct CheckBox<H: 'static> {
    #[core] core: CoreData,
    state: bool,
    label: Option<String>,
    handler: H
}

impl<H> Debug for CheckBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CheckBox {{ core: {:?}, state: {:?}, label: {:?}, handler: <omitted> }}",
            self.core, self.state, self.label)
    }
}

impl<R, H: Fn(bool) -> R> CheckBox<H> {
    /// Construct a checkbox with given `state`, optionally with a `label`,
    /// and with the given `handler`.
    /// 
    /// The handler will be called when the box is checked or unchecked with
    /// the current state (true if checked). It may be a closure like
    /// `|_| MyResponseEnum::ABC` or a stub like `event::noact1`.
    pub fn new(state: bool, label: Option<String>, handler: H) -> Self {
        CheckBox {
            core: Default::default(),
            state, 
            label,
            handler
        }
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
        if let Some(ref s) = self.label {
            &s
        } else {
            ""
        }
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_text(self.tkd(), text);
        self.label = Some(text.into());
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
                (self.handler)(self.state)
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
    handler: H
}

impl<H> Debug for TextButton<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextButton {{ core: {:?}, label: {:?}, handler: <omitted> }}",
            self.core, self.label)
    }
}

impl<R, H: Fn() -> R> TextButton<H> {
    /// Construct a button with a given `label`.
    /// 
    /// The `handler` is called when the button is pressed, and its result is
    /// returned from the event handler. It may be a closure like
    /// `|| MyResponseEnum::ABC` or a stub like `event::noact0`.
    pub fn new<S: Into<String>>(label: S, handler: H) -> Self {
        TextButton {
            core: Default::default(),
            label: label.into(),
            handler
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


impl<R: From<NoResponse>, H: Fn() -> R> Handler for TextButton<H> {
    type Response = R;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::Button => (self.handler)(),
            a @ _ => err_unhandled(a)
        }
    }
}

/// TODO: for use with dialogs...
pub mod button {
    use super::TextButton;
    
    pub fn ok<R, H: Fn() -> R>(handler: H) -> TextButton<H> {
        TextButton::new("Ok", handler)
    }
}
