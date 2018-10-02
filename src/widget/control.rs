//! Basic controls

use std::fmt::{self, Debug};

use crate::event::{self, Action, Handler, ignore};
use crate::widget::{Class, Layout, Widget, CoreData, WidgetCore};
use crate::toolkit::Toolkit;

// TODO: abstract out text part?
#[derive(Clone, Default)]
pub struct TextButton<H> {
    core: CoreData,
    msg: &'static str,
    handler: H,
}

impl_widget_core!(TextButton<H>, core);
impl_layout_simple!(TextButton<H>);

impl<H> Debug for TextButton<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TextButton {{ core: {:?}, msg: {:?}, handler: <omitted> }}",
            self.core, self.msg)
    }
}

impl<H> Widget for TextButton<H> {
    fn class(&self) -> Class { Class::Button }
    fn label(&self) -> Option<&str> { Some(self.msg) }
    
    fn len(&self) -> usize { 0 }
    fn get(&self, index: usize) -> Option<&Widget> { None }
    fn get_mut(&mut self, index: usize) -> Option<&mut Widget> { None }
}

impl<R, H: Fn() -> R> TextButton<H> {
    pub fn new(msg: &'static str, handler: H) -> Self {
        TextButton { core: Default::default(), msg, handler }
    }
}

// impl<H> From<&'static str> for TextButton<event::NoResponse, H>
//     where H: Fn(()) -> event::NoResponse
// {
//     fn from(msg: &'static str) -> Self {
//         TextButton::new(msg, |()| event::NoResponse::None)
//     }
// }


impl<R: From<event::NoResponse>, H: Fn() -> R> Handler for TextButton<H> {
    type Response = R;
    
    fn handle_action(&mut self, _tk: &Toolkit, action: Action, num: u32) -> Self::Response {
        if num != self.get_number() {
            println!("Warning: event passed to wrong widget.");
            return ignore(action);
        }
        
        match action {
            Action::ButtonClick => (self.handler)(),
            a @ _ => ignore(a)
        }
    }
}

pub mod button {
    use super::TextButton;
    
    pub fn ok<R, H: Fn() -> R>(handler: H) -> TextButton<H> {
        TextButton::new("Ok", handler)
    }
}
