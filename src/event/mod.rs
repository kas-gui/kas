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

mod callback;
#[cfg(not(feature = "winit"))]
mod enums;
mod events;
mod manager;
mod response;

use std::fmt::Debug;
// use std::path::PathBuf;

#[cfg(feature = "winit")]
pub use winit::event::{ElementState, ModifiersState, MouseButton, VirtualKeyCode};

use crate::{TkWindow, Widget};

pub use callback::Callback;
#[cfg(not(feature = "winit"))]
pub use enums::*;
pub use events::*;
pub use manager::{HighlightState, Manager};
pub use response::Response;

/// A void message
///
/// This type is not constructible, therefore `Response<VoidMsg>` is known at
/// compile-time not to contain a `Response::Msg(..)` variant.
///
/// Custom message types are required to implement `From<VoidMsg>`. The
/// [`derive(VoidMsg)`](../macros/index.html#the-derivevoidmsg-macro)
/// macro may be used for this purpose.
#[derive(Clone, Debug)]
pub struct VoidMsg;

/// Alias for `Response<VoidMsg>`
pub type VoidResponse = Response<VoidMsg>;

/// Consume an unhandled [`Action`] and return `Response::None`.
///
/// This is an error, meaning somehow an event has been sent to a widget which
/// does not support events of that type.
/// It is safe to ignore this error, but this function panics in debug builds.
pub fn unhandled_action<A: Debug, N>(a: A) -> Response<N> {
    debug_assert!(false, "unhandled_action: {:?}", a);
    Response::None
}

/// Event-handling aspect of a widget.
///
/// This is a companion trait to [`Widget`]. It can (optionally) be implemented
/// by the `derive(Widget)` macro, or can be implemented manually.
///
/// [`Widget`]: crate::Widget
pub trait Handler: Widget {
    /// Type of message returned by this handler.
    ///
    /// This mechanism allows type-safe handling of user-defined responses to handled actions.
    /// For example, a user may define a control panel where each button returns a unique code,
    /// or a configuration editor may return a full copy of the new configuration on completion.
    type Msg;

    /// Handle a high-level event and return a user-defined msg.
    #[inline]
    fn handle_action(&mut self, _: &mut dyn TkWindow, _: Action) -> Response<Self::Msg> {
        Response::None
    }

    /// Handle a low-level event.
    ///
    /// Usually the user has no reason to override the default implementation of
    /// this function. If this is required, it is recommended to handle only the
    /// cases requiring custom handling, and use
    /// [`Manager::handle_generic`] for all other cases.
    #[inline]
    fn handle(&mut self, tk: &mut dyn TkWindow, event: Event) -> Response<Self::Msg> {
        Manager::handle_generic(self, tk, event)
    }
}
