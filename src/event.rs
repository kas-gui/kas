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
// use std::path::PathBuf;

use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

use crate::geom::Coord;
use crate::widget::Core;
use crate::{TkWidget, WidgetId};

/// High-level actions supported by widgets
#[derive(Debug)]
pub enum Action {
    /// Widget activation, for example clicking a button or toggling a check-box
    Activate,
    Dummy, // exists temporarily to allow _ pattern in matchers
}

/// Input events: these are low-level messages where the destination widget is
/// unknown.
///
/// These events are segregated by delivery method.
#[derive(Debug)]
pub enum Event {
    /* NOTE: it's tempting to add this, but we have no model for returning a
     * response from multiple recipients and no use-case.
    /// Events to be addressed to all descendents
    ToAll(EventAll),
    */
    /// Events addressed to a child by number
    ToChild(WidgetId, EventChild),
    /// Events addressed by coordinate
    ToCoord(Coord, EventCoord),
}

/// Events addressed to a child by number
#[derive(Debug)]
pub enum EventChild {
    MouseInput {
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
        modifiers: ModifiersState,
    },
}

/// Events addressed by coordinate
#[derive(Debug)]
pub enum EventCoord {
    CursorMoved {
        device_id: DeviceId,
        modifiers: ModifiersState,
    },
}

// TODO:
//     DroppedFile(PathBuf),
//     HoveredFile(PathBuf),
//     HoveredFileCancelled,
//     ReceivedCharacter(char),
//     Focused(bool),
//     KeyboardInput {
//         device_id: DeviceId,
//         input: KeyboardInput,
//     },
//     CursorEntered {
//         device_id: DeviceId,
//     },
//     CursorLeft {
//         device_id: DeviceId,
//     },
//     MouseWheel {
//         device_id: DeviceId,
//         delta: MouseScrollDelta,
//         phase: TouchPhase,
//         modifiers: ModifiersState,
//     },
//     TouchpadPressure {
//         device_id: DeviceId,
//         pressure: f32,
//         stage: i64,
//     },
//     AxisMotion {
//         device_id: DeviceId,
//         axis: AxisId,
//         value: f64,
//     },
//     Touch(Touch),
//     HiDpiFactorChanged(f64),

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

/// Response type from [`Handler::handle`].
///
/// This type wraps [`Handler::Msg`] allowing both custom messages and toolkit
/// messages.
#[derive(Copy, Clone, Debug)]
pub enum Response<M> {
    /// No action
    None,
    /// Close the window
    Close,
    /// Exit (close all windows)
    Exit,
    /// Custom message type
    Msg(M),
}

// Unfortunately we cannot write generic `From` / `TryFrom` impls
// due to trait coherence rules, so we impl `from` etc. directly.
impl<M> Response<M> {
    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from<N>(r: Response<N>) -> Self
    where
        M: From<N>,
    {
        r.map_into(|msg| Response::Msg(M::from(msg)))
    }

    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be redundant.
    #[inline]
    pub fn into<N>(self) -> Response<N>
    where
        N: From<M>,
    {
        Response::from(self)
    }

    /// Convert from a `Response<()>`
    ///
    /// Once Rust supports specialisation, this will likely be replaced with a
    /// `From` implementation.
    #[inline]
    pub fn from_(r: Response<()>) -> Self {
        r.map_into(|_| Response::None)
    }

    /// Try converting, failing on `Msg` variant
    #[inline]
    pub fn try_from<N>(r: Response<N>) -> Result<Self, N> {
        use Response::*;
        match r {
            None => Ok(None),
            Close => Ok(Close),
            Exit => Ok(Exit),
            Msg(m) => Err(m),
        }
    }

    /// Try converting, failing on `Msg` variant
    #[inline]
    pub fn try_into<N>(self) -> Result<Response<N>, M> {
        Response::try_from(self)
    }

    /// Convert, applying map function on `Msg` variant
    #[inline]
    pub fn map_into<N, F: FnOnce(M) -> Response<N>>(self, op: F) -> Response<N> {
        Response::try_from(self).unwrap_or_else(op)
    }
}

impl Response<()> {
    /// Convert
    ///
    /// Once Rust supports specialisation, this will likely be removed.
    #[inline]
    pub fn into_<N>(self) -> Response<N> {
        Response::from_(self)
    }
}

impl<M> From<M> for Response<M> {
    fn from(msg: M) -> Self {
        Response::Msg(msg)
    }
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
