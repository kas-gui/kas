// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! See documentation of [`Event`] values.
//!
//! ## Event delivery
//!
//! Events can be addressed only to a [`WidgetId`], so the first step (for
//! mouse and touch events) is to use [`crate::Layout::find_id`] to translate a
//! coordinate to a [`WidgetId`].
//!
//! Events are then sent via [`EventMgr::send`] which traverses the widget tree
//! starting from the root (the window), following the path described by the
//! [`WidgetId`]:
//!
//! -   In case any widget encountered is disabled, [`Unused`] is returned
//! -   If the target is found, [`Handler::handle_event`] is called. This method may
//!     handle the event and may push a message to the stack via
//!     [`EventMgr::push_msg`].
//! -   If no target is found, a warning is logged and [`Unused`] returned
//!
//! Then, for each parent back to the root,
//!
//! -   If [`Unused`] was returned, [`Handler::handle_unused`] is called
//! -   Otherwise, if the message stack is non-empty, [`Handler::on_message`]
//!     is called
//!
//! This traversal of the widget tree is fast: (`O(len)`) where `len` is the
//! length of the path described by [`WidgetId`]. It is "clean": uninvolved
//! widgets are not sent any event, while actionable messages are sent to an
//! appropriate parent. It allows "recycling" of unused events.
//!
//! ### Keyboard focus
//!
//! Keyboard focus controls where otherwise undirected keyboard input is sent.
//! This may be set via [`EventMgr::set_nav_focus`] but is typically controlled
//! via the <kbd>Tab</kbd> key, via [`EventMgr::next_nav_focus`].
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

#[cfg(feature = "winit")]
pub use winit::event::{ModifiersState, MouseButton, VirtualKeyCode};
#[cfg(feature = "winit")]
pub use winit::window::CursorIcon;

#[doc(no_inline)]
pub use config::Config;
#[cfg(not(feature = "winit"))]
pub use enums::{CursorIcon, ModifiersState, MouseButton, VirtualKeyCode};
pub use events::*;
pub use handler::Handler;
pub use manager::{EventMgr, EventState, GrabMode};
pub use response::{Response, Scroll};
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

/// A message indicating press focus
///
/// Widgets which are mouse/touch interactible yet do not support keyboard nav
/// focus may return this on [`Event::PressStart`], allowing a parent to take
/// the navigation focus.
#[derive(Clone, Debug, Default)]
pub struct MsgPressFocus;
