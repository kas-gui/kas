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
//! Events can be addressed only to a [`WidgetId`], so the first step (for
//! mouse and touch events) is to use [`kas::Layout::find_id`] to translate a
//! coordinate to a [`WidgetId`].
//!
//! Events then process from root to leaf. [`SendEvent::send`] is responsible
//! for forwarding an event to the appropriate child. Once the target widget is
//! reached, `send` (usually) calls [`Manager::handle_generic`] which may apply
//! some transformations to events, then calls [`Handler::handle`] on target
//! widget. Finally, a [`Response`] is emitted.
//!
//! [`Response`] has three variants: `None`, `Msg(msg)` and `Unhandled(event)`,
//! where `msg` is a message defined by type [`Handler::Msg`] and `event` is an
//! [`Event`]. Both `Msg` and `Unhandled` may be trapped by a parent widget's
//! [`SendEvent::send`], with `Msg` used to trigger a custom action and
//! `Unhandled` used to interpret a few otherwise-unused events (for example,
//! scroll by touch drag).
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

use smallvec::SmallVec;
use std::fmt::Debug;

#[cfg(feature = "winit")]
pub use winit::event::{ModifiersState, MouseButton, VirtualKeyCode};
#[cfg(feature = "winit")]
pub use winit::window::CursorIcon;

pub use callback::Callback;
#[cfg(not(feature = "winit"))]
pub use enums::{CursorIcon, ModifiersState, MouseButton, VirtualKeyCode};
pub use events::*;
pub use handler::{Handler, SendEvent};
pub use manager::{GrabMode, Manager, ManagerState};
pub use response::Response;
pub use update::UpdateHandle;

/// A type supporting a small number of key bindings
///
/// This type may be used where it is desirable to support a small number of
/// key bindings. The type is allowed to silently ignore extra bindings beyond
/// some *small* number of at least 3. (Currently numbers over 5 are accepted
/// but cause allocation.)
pub type VirtualKeyCodes = SmallVec<[VirtualKeyCode; 5]>;

#[test]
fn size_of_virtual_key_codes() {
    // Currently sized to maximise use of available space on 64-bit platforms
    assert!(std::mem::size_of::<VirtualKeyCodes>() <= 32);
}

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
pub enum VoidMsg {}

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
impl_void_msg!(&'static str, String, kas::string::CowString,);
