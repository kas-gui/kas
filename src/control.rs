// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Control widgets accept simple input

use std::fmt::{self, Debug};

use crate::macros::Widget;
use crate::event::{Action, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Core, CoreData, TkWidget};

// TODO: abstract out text part?
#[layout]
#[widget(class = Class::Button, label = Some(self.msg))]
#[derive(Clone, Default, Widget)]
pub struct TextButton<H: 'static> {
    #[core]
    core: CoreData,
    msg: &'static str,
    handler: H,
}

impl<H> Debug for TextButton<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextButton {{ core: {:?}, msg: {:?}, handler: <omitted> }}",
            self.core, self.msg)
    }
}

impl<R, H: Fn() -> R> TextButton<H> {
    pub fn new(msg: &'static str, handler: H) -> Self {
        TextButton { core: Default::default(), msg, handler }
    }
}

// impl<H> From<&'static str> for TextButton<NoResponse, H>
//     where H: Fn(()) -> NoResponse
// {
//     fn from(msg: &'static str) -> Self {
//         TextButton::new(msg, |()| NoResponse)
//     }
// }


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

pub mod button {
    use super::TextButton;
    
    pub fn ok<R, H: Fn() -> R>(handler: H) -> TextButton<H> {
        TextButton::new("Ok", handler)
    }
}
