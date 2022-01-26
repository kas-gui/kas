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
//! mouse and touch events) is to use [`crate::Layout::find_id`] to translate a
//! coordinate to a [`WidgetId`].
//!
//! Events then process from root to leaf. [`SendEvent::send`] is responsible
//! for forwarding an event to the appropriate child. Once the target widget is
//! reached, `send` (usually) calls [`EventMgr::handle_generic`] which may apply
//! some transformations to events, then calls [`Handler::handle`] on target
//! widget. Finally, a [`Response`] is emitted.
//!
//! The [`Response`] enum has a few variants; most important is `Msg(msg)`
//! which passes a user-defined payload up to a parent widget. The
//! `Unused` and `Focus(rect)` variants may be trapped by any parent
//! for secondary purposes, e.g. to adjust a `ScrollRegion`.
//!
//! ## Mouse and touch events
//!
//! Mouse events and touch events are unified: both have a "press" which starts
//! somewhere, moves, and ends somewhere. The main difference concerns move
//! events, which may occur with any number of mouse buttons pressed.
//!
//! Motion and release events are only delivered when a "press grab" is active.
//! This is achieved by calling [`EventMgr::grab_press`] and allows receiving
//! both relative and absolute press coordinates.
//! A special "pan" grab allows receiving two-finger scroll/scale/rotate input.
//!
//! Each touch event is considered independent. The mouse cursor and multiple
//! fingers may all interact with different parts of a UI simultaneously. The
//! same is partly true of keyboard input, though some actions force keyboard
//! focus.
//!
//! ### Pop-ups
//!
//! When a pop-up widget is created, this forces keyboard focus to that widget
//! and receives a "weak" grab on press actions, meaning that the widget
//! receives this input first, but if returned via `Response::Unused` the
//! input passes immediately to the next target. This allows pop-up menus to
//! get first chance of handling input and to dismiss themselves when input is
//! for other widgets without blocking other widgets from accepting that input.
//! (This "weak grab" behaviour is intentional to align UI response with a
//! user's intuition that any visible non-grey part of the UI is interactive.)
//!
//! [`WidgetId`]: crate::WidgetId

pub mod config;
#[cfg(not(feature = "winit"))]
mod enums;
mod events;
mod handler;
mod manager;
mod response;
mod update;

pub mod components;

use smallvec::SmallVec;
use std::fmt::Debug;

// doc imports
#[allow(unused)]
use crate::{theme::InputState, Layout, WidgetCore};

#[cfg(feature = "winit")]
pub use winit::event::{ModifiersState, MouseButton, VirtualKeyCode};
#[cfg(feature = "winit")]
pub use winit::window::CursorIcon;

#[doc(no_inline)]
pub use config::Config;
#[cfg(not(feature = "winit"))]
pub use enums::{CursorIcon, ModifiersState, MouseButton, VirtualKeyCode};
pub use events::*;
pub use handler::{Handler, SendEvent};
pub use manager::{ConfigureManager, EventMgr, EventState, GrabMode};
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
/// [`derive(VoidMsg)`](https://docs.rs/kas/latest/kas/macros#the-derivevoidmsg-macro) macro.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    ($t:ty, $($tt:ty),*) => {
        impl_void_msg!($t);
        impl_void_msg!($($tt),*);
    };
}
impl_void_msg!(bool, char);
impl_void_msg!(u8, u16, u32, u64, u128, usize);
impl_void_msg!(i8, i16, i32, i64, i128, isize);
impl_void_msg!(f32, f64);
impl_void_msg!(&'static str, String);
impl_void_msg!(std::time::Duration, std::time::Instant);

/// A keyed message from a child
///
/// This type is used by some containers to forward messages from children.
#[derive(Clone, Debug)]
pub enum ChildMsg<K, M> {
    Select(K),
    Deselect(K),
    Child(K, M),
}

impl<K, M> From<VoidMsg> for ChildMsg<K, M> {
    fn from(_: VoidMsg) -> Self {
        unreachable!()
    }
}

/// Convert Response<ChildMsg<_, M>> to Response<M>
///
/// `ChildMsg::Child(_, msg)`  translates to `Response::Msg(msg)`; other
/// variants translate to `Response::Used`.
impl<K, M> From<Response<ChildMsg<K, M>>> for Response<M> {
    fn from(r: Response<ChildMsg<K, M>>) -> Self {
        match Response::try_from(r) {
            Ok(r) => r,
            Err(msg) => match msg {
                ChildMsg::Child(_, msg) => Response::Msg(msg),
                _ => Response::Used,
            },
        }
    }
}
