//! Window widgets

use event::{self, Handler};
use widget::{Class, Layout, Widget, CoreData};
use widget::control::{button, TextButton};

/// A window is a drawable interactive region provided by windowing system.
pub trait Window: Widget {
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

impl<W: Widget + 'static> Widget for SimpleWindow<W> {
    fn class(&self) -> Class { Class::Window }
    fn label(&self) -> Option<&str> { None }
    
    fn len(&self) -> usize { 1 }
    fn get(&self, index: usize) -> Option<&(dyn Widget + 'static)> {
        match index {
            0 => Some(&self.w),
            _ => None,
        }
    }
}

impl<R, W: Handler<Response = R> + Widget + 'static> Window for SimpleWindow<W>
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
    core: CoreData,
    message: M,
    button: TextButton<H>,
}

impl<M, R, H: Fn() -> R> MessageBox<M, H> {
    // TODO: action parameter shouldn't be necessary, but we need it because
    // H must be derived from function input somehow, not merely unspecified
    // Once existential types are available, H parameter will not be needed.
    pub fn new(message: M, action: H) -> Self {
        MessageBox{
            core: Default::default(),
            message,
            button: button::ok(action)
        }
    }
}

impl_widget_core!(MessageBox<M, H>, core);

impl<M, H> Layout for MessageBox<M, H> {
    fn min_size(&self) -> (i32, i32) {
        unimplemented!()
    }

    fn set_size(&mut self, size: (i32, i32)) {
        unimplemented!()
    }
}

impl<M, H> Widget for MessageBox<M, H> {
    fn class(&self) -> Class { Class::Window }
    fn label(&self) -> Option<&str> { None }
    
    fn len(&self) -> usize { 0 }
    fn get(&self, index: usize) -> Option<&(dyn Widget + 'static)> {
        unimplemented!()
    }
}

impl<M, H> Window for MessageBox<M, H> {
    fn handle(&mut self, event: event::Event) -> event::Response {
        unimplemented!()
    }
}
