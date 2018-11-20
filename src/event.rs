//! Event handling
//! 
//! Event handling uses *event* messages, passed from the parent into a widget,
//! with responses passed back to the parent. This model is simpler than that
//! commonly used by GUI frameworks: widgets do not need a pointer to their
//! parent and any result is pushed back up the call stack. The model allows
//! type-safety while allowing user-defined result types.

use std::fmt::Debug;
use crate::TkWidget;
use crate::macros::NoResponse;

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

/// No message
/// 
/// All response message types should implement `From<NoResponse>`.
#[derive(Debug)]
pub struct NoResponse;

/// General GUI event responses
#[derive(Debug, NoResponse)]
pub enum GuiResponse {
    /// No action
    None,
    /// Close the window
    Close,
    /// Exit (close all windows)
    Exit,
}

/// Mark explicitly ignored events.
/// 
/// Ignoring events is allowed but emits a warning if enabled.
pub fn ignore<M: Debug, R: From<NoResponse>>(m: M) -> R {
    println!("Handler ignores: {:?}", m);
    NoResponse.into()
}

/// Event-handling aspect of a widget.
pub trait Handler {
    /// Type of message returned by this handler.
    /// 
    /// This mechanism allows type-safe handling of user-defined responses to handled actions.
    /// For example, a user may define a control panel where each button returns a unique code,
    /// or a configuration editor may return a full copy of the new configuration on completion.
    type Response: From<NoResponse>;
    
    /// Handle a high-level event directed at the widget identified by `number`,
    /// and return a user-defined msg.
    fn handle_action(&mut self, tk: &TkWidget, action: Action, number: u32) -> Self::Response {
        let _unused = (tk, number);  // squelch warning
        ignore(action)
    }
}
