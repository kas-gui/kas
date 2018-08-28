//! Window widgets

use widget::{Widget, WidgetCore};
use widget::event;
use widget::control::{button, TextButton};
use widget::layout::WidgetLayout;

/// Main window trait
pub trait Window {
    /// Display the window
    fn display(&mut self);
}

/// Main window type
pub struct SimpleWindow<W> {
    w: W
}

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow { w }
    }
}

impl<W> Window for SimpleWindow<W> {
    fn display(&mut self) {
        // TODO
    }
}

impl<W: WidgetLayout> WidgetLayout for SimpleWindow<W> {
    fn min_size(&self) -> (u32, u32) {
        self.w.min_size()
    }

    fn set_size(&mut self, size: (u32, u32)) {
        self.w.set_size(size)
    }
}

impl<W: WidgetLayout> WidgetCore for SimpleWindow<W> {}

impl<R, W: Widget<Response = R>> Widget for SimpleWindow<W>
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


pub fn action_close() -> impl Fn() -> event::Response {
    || event::Response::Close
}

pub struct MessageBox<M, H> {
    message: M,
    button: TextButton<H>,
}

impl<M, R, H: Fn() -> R> MessageBox<M, H> {
    // TODO: action parameter shouldn't be necessary, but we need it because
    // H must be derived from function input somehow, not merely unspecified
    // Once existential types are available, H parameter will not be needed.
    pub fn new(message: M, action: H) -> Self {
        MessageBox{
            message,
            button: button::ok(action)
        }
    }
}

impl<M, H> Window for MessageBox<M, H> {
    fn display(&mut self) {
        // TODO
    }
}
