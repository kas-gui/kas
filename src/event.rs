// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

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
use crate::widget::Core;

/// Input actions: these are high-level messages aimed at specific widgets.
#[derive(Debug)]
pub enum Action {
    /// An `Entry` has been activated.
    Activate,
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
/// This is an error, meaning somehow an event has been sent to a widget which
/// does not support events of that type.
/// It is safe to ignore this error, but this function panics in debug builds.
pub fn err_unhandled<M: Debug, R: From<NoResponse>>(m: M) -> R {
    debug_assert!(false, "handle_action: event not handled by widget: {:?}", m);
    println!("handle_action: event not handled by widget: {:?}", m);
    NoResponse.into()
}

/// Notify of an incorrect widget number.
/// 
/// This is an error, meaning somehow an event has been sent to a
/// widget number which is not a child of the initial window/widget.
/// It is safe to ignore this error, but this function panics in debug builds.
pub fn err_num<R: From<NoResponse>>() -> R {
    debug_assert!(false, "handle_action: bad widget number");
    println!("handle_action: bad widget number");
    NoResponse.into()
}

/// Event-handling aspect of a widget.
/// 
/// This is a companion trait to [`Widget`]. It can (optionally) be implemented
/// by the `derive(Widget)` macro, or can be implemented manually.
/// 
/// [`Widget`]: crate::Widget
pub trait Handler: Core {
    /// Type of message returned by this handler.
    /// 
    /// This mechanism allows type-safe handling of user-defined responses to handled actions.
    /// For example, a user may define a control panel where each button returns a unique code,
    /// or a configuration editor may return a full copy of the new configuration on completion.
    type Response: From<NoResponse>;
    
    /// Handle a high-level event directed at the widget identified by `number`,
    /// and return a user-defined msg.
    fn handle_action(&mut self, tk: &TkWidget, action: Action, number: u32) -> Self::Response;
}
