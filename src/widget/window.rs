//! Window widgets

use widget::{Widget, WidgetCore};
use widget::event;
use widget::control::{button, TextButton};
use widget::layout::WidgetLayout;

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

impl<W: WidgetLayout> WidgetLayout for Window<W> {
    fn min_size(&self) -> (u32, u32) {
        self.w.min_size()
    }

    fn set_size(&mut self, size: (u32, u32)) {
        self.w.set_size(size)
    }
}

impl<W: WidgetLayout> WidgetCore for Window<W> {}

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
    
    // TODO: use a Window trait? Or re-use the Window type?
    pub fn display(&mut self) {
        //TODO
    }
}
