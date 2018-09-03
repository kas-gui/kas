//! Window widgets

use event::{self, Handler};
use widget::{Layout, Widget, CoreData};
use widget::control::{button, TextButton};

/// A window is a drawable interactive region provided by windowing system.
pub trait Window {
    /// Handle an input event.
    fn handle(&mut self, event: event::Event) -> event::Response;
}

/// Main window type
#[derive(Clone, Default)]
pub struct SimpleWindow<W> {
    core: CoreData,
    w: W
}

impl_widget_core!(SimpleWindow<W>, core);

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow { core: Default::default(), w }
    }
}

impl<W: Layout> Layout for SimpleWindow<W> {
    fn min_size(&self) -> (i32, i32) {
        self.w.min_size()
    }

    fn set_size(&mut self, size: (i32, i32)) {
        self.w.set_size(size)
    }
}

impl<R, W: Handler<Response = R>> Window for SimpleWindow<W>
    where event::Response: From<R>, R: From<event::NoResponse>
{
    fn handle(&mut self, event: event::Event) -> event::Response {
        event::Response::from(self.w.handle(event))
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
    fn handle(&mut self, event: event::Event) -> event::Response {
        unimplemented!()
    }
}
