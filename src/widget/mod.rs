//! Widgets

pub mod canvas;
pub mod control;
pub mod event;
pub mod layout;
pub mod window;


/// Widget trait
pub trait Widget {
    type Response: From<event::NoResponse>;
    
    /// Handle an event, and return a user-defined message
    fn event(&mut self, event: event::Event) -> Self::Response {
        event::ignore(event)
    }
}
