//! Basic controls

use super::event;
use super::{Widget, WidgetCore};
use super::layout::WidgetLayout;

// TODO: abstract out text part?
pub struct TextButton<H> {
    msg: &'static str,
    handler: H,
    size: (u32, u32),
}

impl<R, H: Fn() -> R> TextButton<H> {
    pub fn new(msg: &'static str, handler: H) -> Self {
        TextButton { msg, handler, size: (0, 0) }
    }
}

// impl<H> From<&'static str> for TextButton<event::NoResponse, H>
//     where H: Fn(()) -> event::NoResponse
// {
//     fn from(msg: &'static str) -> Self {
//         TextButton::new(msg, |()| event::NoResponse::None)
//     }
// }


impl<H> WidgetLayout for TextButton<H> {
    fn min_size(&self) -> (u32, u32) {
        (50, 20)    // TODO
    }

    fn set_size(&mut self, size: (u32, u32)) {
        self.size = size;
    }
}

impl<H> WidgetCore for TextButton<H> {}

impl<R: From<event::NoResponse>, H: Fn() -> R> Widget for TextButton<H> {
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
