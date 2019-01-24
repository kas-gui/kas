// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Core, CoreData, HasText, TkWidget};

/// A push-button with a text label
// TODO: abstract out text part?
#[widget(class = Class::Button(self))]
#[derive(Clone, Default, Widget)]
pub struct TextButton<H: 'static> {
    #[core]
    core: CoreData,
    text: String,
    handler: H,
}

impl<H> Debug for TextButton<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextButton {{ core: {:?}, text: {:?}, handler: <omitted> }}",
            self.core, self.text)
    }
}

impl<R, H: Fn() -> R> TextButton<H> {
    /// Construct a button with a given `text` label.
    /// 
    /// The `handler` is called when the button is pressed, and its result is
    /// returned from the event handler.
    pub fn new<S: Into<String>>(text: S, handler: H) -> Self {
        TextButton {
            core: Default::default(),
            text: text.into(),
            handler
        }
    }
}

// impl<H> From<&'static str> for TextButton<NoResponse, H>
//     where H: Fn(()) -> NoResponse
// {
//     fn from(text: &'static str) -> Self {
//         TextButton::new(text, |()| NoResponse)
//     }
// }

impl<H> HasText for TextButton<H> {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
        self.text = text.into();
    }
}


impl<R: From<NoResponse>, H: Fn() -> R> Handler for TextButton<H> {
    type Response = R;
    
    fn handle_action(&mut self, _tk: &TkWidget, action: Action, num: u32) -> Self::Response {
        if num != self.number() {
            return err_num()
        }
        
        match action {
            Action::ButtonClick => (self.handler)(),
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
