// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

use linear_map::{LinearMap, set::LinearSet};
pub(crate) use press::{Mouse, Touch};
use smallvec::SmallVec;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::time::Instant;

use super::*;
use crate::cast::Cast;
use crate::config::WindowConfig;
use crate::geom::{Rect, Size};
use crate::messages::Erased;
use crate::runner::{MessageStack, Platform, RunnerT, WindowDataErased};
use crate::window::{PopupDescriptor, WindowId};
use crate::{Action, Id, Node};
use key::{AccessLayer, PendingSelFocus};
use nav::PendingNavFocus;

mod config;
mod cx_pub;
mod key;
mod nav;
mod platform;
mod press;
mod send;
mod window;

pub use config::ConfigCx;
pub use press::{GrabBuilder, GrabMode, Press, PressSource};

struct PopupState {
    id: WindowId,
    desc: PopupDescriptor,
    old_nav_focus: Option<Id>,
    is_sized: bool,
}

/// Event context state
///
/// This struct encapsulates window-specific event-handling state and handling.
/// Most operations are only available via a [`EventCx`] handle, though some
/// are available on this struct.
///
/// Besides event handling, this struct also configures widgets.
///
/// Some methods are intended only for usage by graphics and platform backends
/// and are hidden from generated documentation unless the `internal_doc`
/// feature is enabled. Event handling assumes [winit].
///
/// [winit]: https://github.com/rust-windowing/winit
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
pub struct EventState {
    pub(crate) window_id: WindowId,
    config: WindowConfig,
    platform: Platform,
    disabled: Vec<Id>,
    window_has_focus: bool,
    #[cfg(feature = "accesskit")]
    accesskit_is_enabled: bool,
    modifiers: ModifiersState,
    /// Key (and IME) focus is on same widget as sel_focus; otherwise its value is ignored
    key_focus: bool,
    ime: Option<ImePurpose>,
    old_ime_target: Option<Id>,
    /// Rect is cursor area in sel_focus's coordinate space if size != ZERO
    ime_cursor_area: Rect,
    last_ime_rect: Rect,
    sel_focus: Option<Id>,
    nav_focus: Option<Id>,
    nav_fallback: Option<Id>,
    key_depress: LinearMap<PhysicalKey, Id>,
    mouse: Mouse,
    touch: Touch,
    access_layers: BTreeMap<Id, AccessLayer>,
    popups: SmallVec<[PopupState; 16]>,
    popup_removed: SmallVec<[(Id, WindowId); 16]>,
    time_updates: Vec<(Instant, Id, TimerHandle)>,
    frame_updates: LinearSet<(Id, TimerHandle)>,
    need_frame_update: bool,
    // Set of messages awaiting sending
    send_queue: VecDeque<(Id, Erased)>,
    // Set of futures of messages together with id of sending widget
    fut_messages: Vec<(Id, Pin<Box<dyn Future<Output = Erased>>>)>,
    // Widget requiring update
    pending_update: Option<Id>,
    // Optional new target for selection focus. bool is true if this also gains key focus.
    pending_sel_focus: Option<PendingSelFocus>,
    pending_nav_focus: PendingNavFocus,
    pub(crate) action: Action,
}

impl EventState {
    /// Clear all focus and grabs on `target`
    fn cancel_event_focus(&mut self, target: &Id) {
        self.clear_sel_socus_on(target);
        self.clear_nav_focus_on(target);
        self.mouse.cancel_event_focus(target);
        self.touch.cancel_event_focus(target);
    }
}

/// Event handling context
///
/// `EventCx` and [`EventState`] (available via [`Deref`]) support various
/// event management and event-handling state querying operations.
#[must_use]
pub struct EventCx<'a> {
    state: &'a mut EventState,
    runner: &'a mut dyn RunnerT,
    window: &'a dyn WindowDataErased,
    messages: &'a mut MessageStack,
    pub(crate) target_is_disabled: bool,
    last_child: Option<usize>,
    scroll: Scroll,
}

impl<'a> Deref for EventCx<'a> {
    type Target = EventState;
    fn deref(&self) -> &Self::Target {
        self.state
    }
}
impl<'a> DerefMut for EventCx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}
