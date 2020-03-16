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
//!
//! We deliver events only on a "need to know" basis: typically, only one widget
//! will receive an event.
//!
//! ## Event delivery
//!
//! Events are processed from root to leaf, targetted either at a coordinate or
//! at a [`WidgetId`]. Events targetted at coordinates are translated to a
//! [`WidgetId`] by [`Manager::handle_generic`]; events targetted at a
//! [`WidgetId`] are handled by widget-specific code or returned via
//! [`Response::Unhandled`], in which case any caller may handle the event.
//!
//! ## Mouse and touch events
//!
//! Mouse events and touch events are unified: both have a "press" which starts
//! somewhere, moves, and ends somewhere. The main difference concerns move
//! events, which may occur with any number of mouse buttons pressed.
//!
//! Each touch event is considered independent, allowing multiple fingers to
//! interact with a UI simultaneously; only where the same widget receives
//! multiple events can multi-finger gestures be processed. In contrast, mouse
//! events are considered to come from a single mouse, and when a mouse-grab is
//! in effect, all mouse events are delivered to the grabbing widget.
//!
//! Press-start events are delivered to the widget at the cursor/touch location,
//! with the exception of mouse events when a mouse grab is already in effect.
//! If unhandled, the event is passed up to parent widgets who may choose to
//! handle the event. The first widget processing the event may request a grab
//! on the touch/mouse event in order to receive motion and press-end events.
//! The grab automatically ends after the corresponding press-end event.
//!
//! Motion events are delivered to whichever widget has a grab on the touch
//! event or the mouse. If no grab is enabled, such events are not delivered.
//!
//! Press-end events are delivered to whichever widget has a grab on the touch
//! event or the mouse; otherwise (if no grab affects this event), the event is
//! delivered to the widget at the event coordinates.
//!
//! Widgets should not normally need internal tracking of mouse/touch events.
//! Highlighting information can be obtained directly in the `draw` method, and
//! press events provide information on their start and end widget.
//!
//! [`WidgetId`]: crate::WidgetId

mod callback;
#[cfg(not(feature = "winit"))]
mod enums;
mod events;
mod handler;
mod manager;
mod response;
mod update;

use std::fmt::Debug;
// use std::path::PathBuf;

#[cfg(feature = "winit")]
pub use winit::event::{MouseButton, VirtualKeyCode};
#[cfg(feature = "winit")]
pub use winit::window::CursorIcon;

pub use callback::Callback;
#[cfg(not(feature = "winit"))]
pub use enums::{CursorIcon, MouseButton, VirtualKeyCode};
pub use events::*;
pub use handler::Handler;
pub use manager::{GrabMode, HighlightState, Manager, ManagerState};
pub use response::Response;
pub use update::UpdateHandle;

/// A void message
///
/// This type is not constructible, therefore `Response<VoidMsg>` is known at
/// compile-time not to contain a `Response::Msg(..)` variant.
///
/// It is trivial to implement `From<VoidMsg>` for any type `T`; unfortunately
/// Rust's type system is too restrictive for us to provide a blanket
/// implementation (due both to orphan rules for trait implementations and to
/// conflicting implementations; it is possible that this may change in the
/// future).
///
/// `From<VoidMsg>` is implemented for a number of language types;
/// custom message types are required to implement this via the
/// [`derive(VoidMsg)`](../macros/index.html#the-derivevoidmsg-macro) macro.
#[derive(Clone, Debug)]
pub struct VoidMsg;

/// Alias for `Response<VoidMsg>`
pub type VoidResponse = Response<VoidMsg>;

// TODO(specialization): replace below impls with impl<T> From<VoidMsg> for T
macro_rules! impl_void_msg {
    () => {};
    ($t:ty) => {
        impl From<VoidMsg> for $t {
            fn from(_: VoidMsg) -> $t {
                unreachable!()
            }
        }
    };
    ($t:ty, $($tt:ty,)*) => {
        impl_void_msg!($t);
        impl_void_msg!($($tt,)*);
    };
}
impl_void_msg!(bool, char,);
impl_void_msg!(u8, u16, u32, u64, u128, usize,);
impl_void_msg!(i8, i16, i32, i64, i128, isize,);
impl_void_msg!(f32, f64,);
impl_void_msg!(&'static str, String, kas::CowString,);
