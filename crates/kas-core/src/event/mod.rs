// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! ## Event handling model
//!
//! Note: widgets are represented as an acyclic tree, with the *root* at the
//! "top" of the tree. Each tree node is a [`Widget`] and has a [`WidgetId`].
//! A [`WidgetId`] represents a *path* and may be used to find the most
//! direct root from the root to the target.
//!
//! An [`Event`] is [sent](EventMgr::send) to a target widget as follows:
//!
//! 1.  Determine the target's [`WidgetId`]. For example, this may be
//!     the [`nav_focus`](EventState::nav_focus) or may be determined from
//!     from mouse/touch coordinates by calling [`find_id`](crate::Layout::find_id).
//! 2.  If the target is [disabled](EventState::is_disabled), then find the
//!     top-most ancestor which is disabled and make that the target, but
//!     inhibit calling of [`Widget::handle_event`] (thus
//!     [`Widget::handle_unused`] is still called on all non-disabled ancestors).
//! 3.  Traverse *down* the widget tree from its root to the target according to
//!     the [`WidgetId`]. On each node (excluding the target),
//!
//!     -   Call [`Widget::steal_event`]; if this method "steals" the event,
//!         skip to step 5.
//! 4.  In the normal case (when the target is not disabled and the event is
//!     not stolen), [`Widget::handle_event`] is called on the target.
//! 5.  If the message stack is not empty, call [`Widget::handle_message`] on
//!     the current node.
//! 6.  Unwind, traversing back *up* the widget tree (towards the root).
//!     On each node (excluding the target),
//!
//!     -   If a non-empty scroll action is [set](EventMgr::set_scroll),
//!         call [`Widget::handle_scroll`]
//!     -   If the event has not yet been [used](Response::Used),
//!         call [`Widget::handle_unused`]
//!     -   If the message stack is non-empty (see [`EventMgr::push`]),
//!         call [`Widget::handle_message`].
//! 7.  Clear any messages still on the message stack, printing a warning to the
//!     log. Messages *should* be handled during unwinding, though not doing so
//!     is safe (and possibly useful during development).
//!
//! ### Pop-ups
//!
//! When a pop-up widget is created, the pop-up's parent takes priority for
//! "press" (mouse / touch) input as well as receiving keyboard focus.
//!
//! If this input is unhandled, the pop-up is automatically closed and the event
//! is re-sent to the next candidate, allowing handling of e.g. mouse clicks on
//! widgets under a menu. This should be intuitive: UI which is in focus and
//! not greyed-out should be interactive.
//!
//! [`WidgetId`]: crate::WidgetId
//! [`Unused`]: Response::Unused

pub mod config;
#[cfg(not(feature = "winit"))] mod enums;
mod events;
mod manager;
mod response;
mod update;

pub mod components;

use smallvec::SmallVec;
use std::fmt::Debug;

#[cfg(feature = "winit")]
pub use winit::event::{ModifiersState, MouseButton, VirtualKeyCode};
#[cfg(feature = "winit")] pub use winit::window::CursorIcon;

#[allow(unused)] use crate::Widget;
#[doc(no_inline)] pub use config::Config;
#[cfg(not(feature = "winit"))]
pub use enums::{CursorIcon, ModifiersState, MouseButton, VirtualKeyCode};
pub use events::*;
pub use manager::{ConfigMgr, EventMgr, EventState, GrabBuilder, GrabMode, Press};
pub use response::{Response, Scroll};
pub use update::UpdateId;

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
