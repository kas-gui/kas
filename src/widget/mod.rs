//! Widgets

pub mod canvas;
pub mod control;
pub mod event;
pub mod layout;
pub mod window;


/// Core widget trait (object-safe)
pub trait WidgetCore {
}

/// Widget trait — includes user-customisable sub-type
pub trait Widget: WidgetCore {
    type Response: From<event::NoResponse>;
    
    /// Handle an event, and return a user-defined message
    fn handle(&mut self, event: event::Event) -> Self::Response {
        event::ignore(event)
    }
}
