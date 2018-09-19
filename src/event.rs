//! Event handling
//! 
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent.

/// Input actions: these are high-level events aimed at specific widgets.
#[derive(Debug)]
pub enum Action {
    /// A button has been clicked.
    ButtonClick,
    /// A window has been asked to close.
    Close,
}

// TODO: pub enum Event { .. }

/// A simple response message with only a single variant.
/// 
/// All response message types should implement `From<NoResponse>`.
pub enum NoResponse {
    /// No action
    None,
}

/// Mark explicitly ignored events.
/// 
/// Ignoring events is allowed but emits a warning if enabled.
pub fn ignore<M: From<NoResponse>>(a: Action) -> M {
    println!("Ignored action: {:?}", a);
    M::from(NoResponse::None)
}

/// Event-handling aspect of a widget.
pub trait Handler {
    type Response: From<NoResponse>;
    
    /// Handle a high-level event directed at the widget identified by `num`,
    /// and return a user-defined message.
    fn handle_action(&mut self, action: Action, num: u32) -> Self::Response {
        ignore(action)
    }
}
