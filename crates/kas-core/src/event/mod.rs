// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! ## Event handling model
//!
//! 1.  Determine the target [`WidgetId`]. This is dependant on the type of
//!     event and may (non-exhaustively) derive
//!     from keyboard navigation (see [`EventState::set_nav_focus`], [`EventMgr::next_nav_focus`]),
//!     from key bindings,
//!     from mouse/touch coordinates (see [`crate::Layout::find_id`])
//!     or from a mouse/touch grab ([`EventMgr::grab_press`]).
//! 2.  Construct an [`Event`].
//! 3.  Use [`EventMgr::send`] to traverse the widget tree from its root to the
//!     target.
//!
//!     In case a widget [`EventState::is_disabled`], then traversal of the
//!     widget tree may stop early, depending on [`Event::pass_when_disabled`].
//!     Otherwise, call [`Widget::steal_event`] on each ancestor, stopping early
//!     if any returns [`Response::Used`]. In both cases the return traversal of
//!     step 4 still happens, but only from the widget stopped at here.
//!
//!     In the normal case, [`Widget::handle_event`] is called on the target.
//! 4.  Traverse back towards the widget tree's root.
//!
//!     If no handler has yet returned [`Response::Used`], then call
//!     [`Widget::handle_unused`] on each ancestor visited.
//!
//!     If the message stack is non-empty (due to a handler calling [`EventMgr::push`]),
//!     call [`Widget::handle_message`] on each ancestor visited.
//!
//!     If a non-empty scroll action is set (due to a handler calling [`EventMgr::set_scroll`]),
//!     call [`Widget::handle_scroll`] on each ancestor visited.
//! 5.  If the message stack is non-empty on reaching the root widget then log a
//!     warning (including the formatted message in debug builds).
//!     It is expected that for any message a widget might push to the stack,
//!     *some* ancestor will check for and handle this message (not doing so is
//!     safe but probably means that some control is not operational).
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
pub use manager::{ConfigMgr, ErasedMessage, EventMgr, EventState, GrabMode};
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
