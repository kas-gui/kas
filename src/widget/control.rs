//! Basic controls

use event;
use widget::{Layout, Widget, CoreData};

// TODO: abstract out text part?
#[derive(Clone, Default)]
pub struct TextButton<H> {
    core: CoreData,
    msg: &'static str,
    handler: H,
}

impl_widget_core!(TextButton<H>, core);
impl_leaf_widget!(TextButton<H>);

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


impl<H> Layout for TextButton<H> {
    fn min_size(&self) -> (i32, i32) {
        (50, 20)    // TODO
    }
}

impl<R: From<event::NoResponse>, H: Fn() -> R> event::Handler for TextButton<H> {
    type Response = R;
    
    fn handle(&mut self, event: event::Event) -> Self::Response {
        if false /* TODO */ {
            (self.handler)()
        } else {
            event::NoResponse::None.into()
        }
    }
}

pub mod button {
    use super::TextButton;
    
    pub fn ok<R, H: Fn() -> R>(handler: H) -> TextButton<H> {
        TextButton::new("Ok", handler)
    }
}
