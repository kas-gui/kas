//! Basic controls

use super::event;
use super::Widget;

// TODO: abstract out text part?
// TODO: use abstraction around handler to allow configuration with single template parameter?
pub struct TextButton<R, H: Fn() -> R> {
    msg: &'static str,
    handler: H,
}

impl<R, H: Fn() -> R> TextButton<R, H> {
    pub fn new(msg: &'static str, handler: H) -> Self {
        TextButton { msg, handler }
    }
}

// impl<H> From<&'static str> for TextButton<event::NoResponse, H>
//     where H: Fn(()) -> event::NoResponse
// {
//     fn from(msg: &'static str) -> Self {
//         TextButton::new(msg, |()| event::NoResponse::None)
//     }
// }

impl<R: From<event::NoResponse>, H: Fn() -> R> Widget for TextButton<R, H> {
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
    
    pub fn ok<R, H: Fn() -> R>(handler: H) -> TextButton<R, H> {
        TextButton::new("Ok", handler)
    }
}
