//! Event handling
//! 
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent. This model is simpler than that
//! commonly used by GUI frameworks: widgets do not need a pointer to their
//! parent and any result is pushed back up the call stack. The model allows
//! type-safety while allowing user-defined result types.

use crate::toolkit::Toolkit;

/// Input actions: these are high-level messages aimed at specific widgets.
#[derive(Debug)]
pub enum Action {
    /// A button has been clicked.
    ButtonClick,
    /// A window has been asked to close.
    Close,
}

/*
/// Input events: these are low-level messages where the destination widget is
/// unknown.
/// 
/// TODO: probably just re-export `winit::Event`, maybe behind a feature flag.
#[derive(Debug)]
pub type Event = ();
*/

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
    println!("Handler ignores: {:?}", a);
    M::from(NoResponse::None)
}

/// Event-handling aspect of a widget.
pub trait Handler {
    /// Type of response from handlers. This allows type-safe handling of
    /// responses to handled actions. Various widgets allow sending responses of
    /// user-defined type, but only user-defined widgets can handle these
    /// responses.
    type Response: From<NoResponse>;
    
    /// Handle a high-level event directed at the widget identified by `number`,
    /// and return a user-defined message.
    fn handle_action(&mut self, tk: &Toolkit, action: Action, number: u32)
        -> Self::Response
    {
        let _unused = (tk, number);  // squelch warning
        ignore(action)
    }
}
