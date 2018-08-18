//! Window widgets

use widget::event;
use super::{Widget, control::button, layout};

/// Main window type
pub struct Window<W: Widget> {
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

impl<W: Widget> Widget for Window<W> {
    type Response = event::NoResponse;
    
    fn event(&mut self, event: event::Event) -> Self::Response {
        unimplemented!()
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
