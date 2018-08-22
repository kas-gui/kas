//! Window widgets

use widget::event;
use super::{Widget, WidgetCore};
use super::{control::button, layout};

/// Main window type
pub struct Window<W> {
    w: W
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new(w: W) -> Window<W> {
        Window { w }
    }
    
    /// Display the window
    pub fn display(&mut self) {
        // TODO
    }
}

impl<W> WidgetCore for Window<W> {}

impl<R, W: Widget<Response = R>> Widget for Window<W>
    where event::Response: From<R>, R: From<event::NoResponse>
{
    type Response = event::NoResponse;
    
    fn handle(&mut self, event: event::Event) -> Self::Response {
        let response = event::Response::from(self.w.handle(event));
        match response {
            event::Response::None => event::NoResponse::None,
            event::Response::Close => {
                unimplemented!()
            }
        }
    }
}


pub fn message_box<M: Widget>(message: M) -> Window<impl Widget> {
    Window::new(
        layout::VList2::new(
            message,
            button::ok(|| event::Response::Close)
        )
    )
}
