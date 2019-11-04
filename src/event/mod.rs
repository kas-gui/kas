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
mod events;
mod response;

use std::fmt::Debug;
// use std::path::PathBuf;

use winit::event::{ElementState, MouseButton};

use crate::{Core, TkWidget};

pub use callback::Callback;
pub use events::*;
pub use response::Response;

/// Mark explicitly ignored events.
///
/// This is an error, meaning somehow an event has been sent to a widget which
/// does not support events of that type.
/// It is safe to ignore this error, but this function panics in debug builds.
pub fn err_unhandled<M: Debug, N>(m: M) -> Response<N> {
    debug_assert!(
        false,
        "Handler::handle: event not handled by widget: {:?}",
        m
    );
    println!("Handler::handle: event not handled by widget: {:?}", m);
    Response::None
}

/// Notify of an incorrect widget number.
///
/// This is an error, meaning somehow an event has been sent to a
/// widget number which is not a child of the initial window/widget.
/// It is safe to ignore this error, but this function panics in debug builds.
pub fn err_num<N>() -> Response<N> {
    debug_assert!(false, "Handler::handle: bad widget number");
    println!("Handler::handle: bad widget number");
    Response::None
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
    type Msg;

    /// Handle a high-level event and return a user-defined msg.
    fn handle_action(&mut self, _: &mut dyn TkWidget, _: Action) -> Response<Self::Msg> {
        Response::None
    }

    /// Handle a low-level event. Normally the user should not override this.
    fn handle(&mut self, tk: &mut dyn TkWidget, event: Event) -> Response<Self::Msg> {
        let self_id = Some(self.number());
        match event {
            Event::ToChild(_, ev) => match ev {
                EventChild::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                tk.set_click_start(self_id);
                                Response::None
                            }
                            ElementState::Released => {
                                let r = if tk.click_start() == self_id {
                                    self.handle_action(tk, Action::Activate)
                                } else {
                                    Response::None
                                };
                                tk.set_click_start(None);
                                r
                            }
                        }
                    } else {
                        Response::None
                    }
                }
            },
            Event::ToCoord(_, ev) => {
                match ev {
                    EventCoord::CursorMoved { .. } => {
                        // We can assume the pointer is over this widget
                        tk.set_hover(self_id);
                        Response::None
                    }
                }
            }
        }
    }
}
