// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling
//!
//! ## Event handling model
//!
//! Note: widgets are represented as an acyclic tree, with the *root* at the
//! "top" of the tree. Each tree node is a [`Widget`] and has an [`Id`].
//! An [`Id`] represents a *path* and may be used to find the most
//! direct root from the root to the target.
//!
//! An [`Event`] is sent to a target widget as follows:
//!
//! 1.  Determine the target's [`Id`]. For example, this may be
//!     the [`nav_focus`](EventState::nav_focus) or may be determined from
//!     from mouse/touch coordinates by calling [`try_probe`](crate::Layout::try_probe).
//! 2.  If the target is [disabled](EventState::is_disabled), then find the
//!     top-most ancestor which is disabled and make that the target, but
//!     inhibit calling of [`Events::handle_event`] on this widget (but still
//!     unwind, calling [`Events::handle_event`] on ancestors)).
//! 3.  Traverse *down* the widget tree from its root to the target according to
//!     the [`Id`].
//! 4.  In the normal case (when the target is not disabled and the event is
//!     not stolen), [`Events::handle_event`] is called on the target.
//! 5.  If the message stack is not empty, call [`Events::handle_messages`] on
//!     the current node.
//! 6.  Unwind, traversing back *up* the widget tree (towards the root).
//!     On each node (excluding the target),
//!
//!     -   If a non-empty scroll action is [set](EventCx::set_scroll),
//!         call [`Events::handle_scroll`]
//!     -   If the event has not yet been [used](Used),
//!         call [`Events::handle_event`]
//!     -   If the message stack is non-empty (see [`EventCx::push`]),
//!         call [`Events::handle_messages`].
//! 7.  If the message stack is not empty, call
//!     [`AppData::handle_messages`](crate::runner::AppData::handle_messages).
//! 8.  Clear any messages still on the message stack, printing a warning to the
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
//! [`Id`]: crate::Id

pub mod components;
mod cx;
mod events;
mod response;

pub use smol_str::SmolStr;
pub use winit::event::{ElementState, KeyEvent, MouseButton};
pub use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};
pub use winit::window::{CursorIcon, ImePurpose, ResizeDirection}; // used by Key

#[allow(unused)] use crate::{Events, Widget};
pub use cx::*;
pub use events::*;
pub use response::{IsUsed, Scroll, Unused, Used};
