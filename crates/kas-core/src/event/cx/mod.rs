// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

use linear_map::{LinearMap, set::LinearSet};
pub(crate) use press::{Mouse, Touch};
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap, VecDeque};
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
use crate::util::WidgetHierarchy;
use crate::{Action, Id, NavAdvance, Node, WindowId};

mod config;
mod cx_pub;
mod platform;
mod press;

pub use config::ConfigCx;
pub use press::{GrabBuilder, GrabMode, Press, PressSource};

#[derive(Debug)]
struct PendingSelFocus {
    target: Option<Id>,
    key_focus: bool,
    ime: Option<ImePurpose>,
    source: FocusSource,
}

#[crate::impl_default(PendingNavFocus::None)]
enum PendingNavFocus {
    None,
    Set {
        target: Option<Id>,
        source: FocusSource,
    },
    Next {
        target: Option<Id>,
        reverse: bool,
        source: FocusSource,
    },
}

type AccessLayer = (bool, HashMap<Key, Id>);

struct PopupState {
    id: WindowId,
    desc: crate::PopupDescriptor,
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
    #[inline]
    fn key_focus(&self) -> Option<Id> {
        if self.key_focus { self.sel_focus.clone() } else { None }
    }

    fn clear_key_focus(&mut self) {
        if self.key_focus {
            if let Some(ref mut pending) = self.pending_sel_focus {
                if pending.target == self.sel_focus {
                    pending.key_focus = false;
                }
            } else {
                self.pending_sel_focus = Some(PendingSelFocus {
                    target: None,
                    key_focus: false,
                    ime: None,
                    source: FocusSource::Synthetic,
                });
            }
        }
    }

    // Remove popup at index and return its [`WindowId`]
    //
    // Panics if `index` is out of bounds.
    //
    // The caller must call `runner.close_window(window_id)`.
    #[must_use]
    fn close_popup(&mut self, index: usize) -> WindowId {
        let state = self.popups.remove(index);
        if state.is_sized {
            self.popup_removed.push((state.desc.id, state.id));
        }
        self.mouse.tooltip_popup_close(&state.desc.parent);

        if let Some(id) = state.old_nav_focus {
            self.set_nav_focus(id, FocusSource::Synthetic);
        }

        state.id
    }

    /// Clear all focus and grabs on `target`
    fn cancel_event_focus(&mut self, target: &Id) {
        if let Some(id) = self.sel_focus.as_ref()
            && target.is_ancestor_of(id)
        {
            if let Some(pending) = self.pending_sel_focus.as_mut() {
                if pending.target.as_ref() == Some(id) {
                    pending.target = None;
                    pending.key_focus = false;
                } else {
                    // We have a new focus target, hence the old one will be cleared
                }
            } else {
                self.pending_sel_focus = Some(PendingSelFocus {
                    target: None,
                    key_focus: false,
                    ime: None,
                    source: FocusSource::Synthetic,
                });
            }
        }

        if let Some(id) = self.nav_focus.as_ref()
            && target.is_ancestor_of(id)
        {
            if matches!(&self.pending_nav_focus, PendingNavFocus::Set { target, .. } if target.as_ref() == Some(id))
            {
                self.pending_nav_focus = PendingNavFocus::None;
            }

            if matches!(self.pending_nav_focus, PendingNavFocus::None) {
                self.pending_nav_focus = PendingNavFocus::Set {
                    target: None,
                    source: FocusSource::Synthetic,
                };
            }
        }

        self.mouse.cancel_event_focus(target);
        self.touch.cancel_event_focus(target);
    }

    pub(crate) fn confirm_popup_is_sized(&mut self, id: WindowId) {
        for popup in &mut self.popups {
            if popup.id == id {
                popup.is_sized = true;
            }
        }
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

impl<'a> EventCx<'a> {
    fn start_key_event(&mut self, mut widget: Node<'_>, vkey: Key, code: PhysicalKey) {
        log::trace!(
            "start_key_event: widget={}, vkey={vkey:?}, physical_key={code:?}",
            widget.id()
        );

        let opt_cmd = self.config.shortcuts().try_match(self.modifiers, &vkey);

        if Some(Command::Exit) == opt_cmd {
            self.runner.exit();
            return;
        } else if Some(Command::Close) == opt_cmd {
            self.handle_close();
            return;
        } else if let Some(cmd) = opt_cmd {
            let mut targets = vec![];
            let mut send = |_self: &mut Self, id: Id, cmd| -> bool {
                if !targets.contains(&id) {
                    let event = Event::Command(cmd, Some(code));
                    let used = _self.send_event(widget.re(), id.clone(), event);
                    targets.push(id);
                    used
                } else {
                    false
                }
            };

            if (self.key_focus || cmd.suitable_for_sel_focus())
                && let Some(id) = self.sel_focus.clone()
                && send(self, id, cmd)
            {
                return;
            }

            if !self.modifiers.alt_key()
                && let Some(id) = self.nav_focus.clone()
                && send(self, id, cmd)
            {
                return;
            }

            if let Some(id) = self
                .popups
                .last()
                .filter(|popup| popup.is_sized)
                .map(|popup| popup.desc.id.clone())
                && send(self, id, cmd)
            {
                return;
            }

            if let Some(id) = self.nav_fallback.clone()
                && send(self, id, cmd)
            {
                return;
            }

            if matches!(cmd, Command::Debug) {
                let over_id = self.mouse.over_id();
                let hier = WidgetHierarchy::new(widget.as_tile(), over_id.clone());
                log::debug!("Widget heirarchy (filter={over_id:?}): {hier}");
                return;
            }
        }

        // Next priority goes to access keys when Alt is held or alt_bypass is true
        let mut target = None;
        for id in (self.popups.iter().rev())
            .filter(|popup| popup.is_sized)
            .map(|state| state.desc.id.clone())
            .chain(std::iter::once(widget.id()))
        {
            if let Some(layer) = self.access_layers.get(&id) {
                // but only when Alt is held or alt-bypass is enabled:
                if (self.modifiers == ModifiersState::ALT
                    || layer.0 && self.modifiers == ModifiersState::empty())
                    && let Some(id) = layer.1.get(&vkey).cloned()
                {
                    target = Some(id);
                    break;
                }
            }
        }

        if let Some(id) = target {
            if let Some(id) = self.nav_next(widget.re(), Some(&id), NavAdvance::None) {
                self.set_nav_focus(id, FocusSource::Key);
            }
            let event = Event::Command(Command::Activate, Some(code));
            self.send_event(widget, id, event);
        } else if self.config.nav_focus && opt_cmd == Some(Command::Tab) {
            let shift = self.modifiers.shift_key();
            self.next_nav_focus_impl(widget.re(), None, shift, FocusSource::Key);
        } else if opt_cmd == Some(Command::Escape)
            && let Some(id) = self.popups.last().map(|desc| desc.id)
        {
            self.close_window(id);
        }
    }

    pub(crate) fn post_send(&mut self, index: usize) -> Option<Scroll> {
        self.last_child = Some(index);
        (self.scroll != Scroll::None).then_some(self.scroll.clone())
    }

    /// Send a few message types as an Event, replay other messages as if pushed by `id`
    ///
    /// Optionally, push `msg` and set `scroll` as if pushed/set by `id`.
    fn send_or_replay(&mut self, mut widget: Node<'_>, id: Id, msg: Erased) {
        if msg.is::<Command>() {
            let cmd = *msg.downcast().unwrap();
            if !self.send_event(widget, id, Event::Command(cmd, None)) {
                match cmd {
                    Command::Exit => self.runner.exit(),
                    Command::Close => self.handle_close(),
                    _ => (),
                }
            }
        } else if msg.is::<ScrollDelta>() {
            let event = Event::Scroll(*msg.downcast().unwrap());
            self.send_event(widget, id, event);
        } else {
            debug_assert!(self.scroll == Scroll::None);
            debug_assert!(self.last_child.is_none());
            self.messages.set_base();
            log::trace!(target: "kas_core::event", "replay: id={id}: {msg:?}");

            self.target_is_disabled = false;
            self.push_erased(msg);
            widget._replay(self, id);
            self.last_child = None;
            self.scroll = Scroll::None;
        }
    }

    /// Replay a scroll action
    #[cfg(feature = "accesskit")]
    fn replay_scroll(&mut self, mut widget: Node<'_>, id: Id, scroll: Scroll) {
        log::trace!(target: "kas_core::event", "replay_scroll: id={id}: {scroll:?}");
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.scroll = scroll;
        self.messages.set_base();

        self.target_is_disabled = false;
        widget._replay(self, id);
        self.last_child = None;
        self.scroll = Scroll::None;
    }

    // Call Widget::_send; returns true when event is used
    fn send_event(&mut self, mut widget: Node<'_>, mut id: Id, event: Event) -> bool {
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.messages.set_base();
        log::trace!(target: "kas_core::event", "send_event: id={id}: {event:?}");

        // TODO(opt): we should be able to use binary search here
        let mut disabled = false;
        if !event.pass_when_disabled() {
            for d in &self.disabled {
                if d.is_ancestor_of(&id) {
                    id = d.clone();
                    disabled = true;
                }
            }
            if disabled {
                log::trace!(target: "kas_core::event", "target is disabled; sending to ancestor {id}");
            }
        }
        self.target_is_disabled = disabled;

        let used = widget._send(self, id, event) == Used;

        self.last_child = None;
        self.scroll = Scroll::None;
        used
    }

    // Closes any popup which is not an ancestor of `id`
    fn close_non_ancestors_of(&mut self, id: Option<&Id>) {
        for index in (0..self.popups.len()).rev() {
            if let Some(id) = id
                && self.popups[index].desc.id.is_ancestor_of(id)
            {
                continue;
            }

            let id = self.close_popup(index);
            self.runner.close_window(id);
        }
    }

    fn handle_close(&mut self) {
        let mut id = self.window_id;
        if !self.popups.is_empty() {
            let index = self.popups.len() - 1;
            id = self.close_popup(index);
        }
        self.runner.close_window(id);
    }

    // Call Widget::_nav_next
    #[inline]
    fn nav_next(
        &mut self,
        mut widget: Node<'_>,
        focus: Option<&Id>,
        advance: NavAdvance,
    ) -> Option<Id> {
        log::trace!(target: "kas_core::event", "nav_next: focus={focus:?}, advance={advance:?}");

        widget._nav_next(&mut self.config_cx(), focus, advance)
    }

    // Set selection focus to `wid` immediately; if `key_focus` also set that
    fn set_sel_focus(
        &mut self,
        window: &dyn WindowDataErased,
        mut widget: Node<'_>,
        pending: PendingSelFocus,
    ) {
        let PendingSelFocus {
            target,
            key_focus,
            ime,
            source,
        } = pending;
        let target_is_new = target != self.sel_focus;
        let old_key_focus = self.key_focus;
        self.key_focus = key_focus;

        log::trace!("set_sel_focus: target={target:?}, key_focus={key_focus}");

        if let Some(id) = self.sel_focus.clone() {
            if self.ime.is_some() && (ime.is_none() || target_is_new) {
                window.set_ime_allowed(None);
                self.old_ime_target = Some(id.clone());
                self.ime = None;
                self.ime_cursor_area = Rect::ZERO;
            }

            if old_key_focus && (!key_focus || target_is_new) {
                // If widget has key focus, this is lost
                self.send_event(widget.re(), id.clone(), Event::LostKeyFocus);
            }

            if target.is_none() {
                // Retain selection focus without a new target
                return;
            } else if target_is_new {
                // Selection focus is lost if another widget receives key focus
                self.send_event(widget.re(), id, Event::LostSelFocus);
            }
        }

        if let Some(id) = target.clone() {
            if target_is_new {
                self.send_event(widget.re(), id.clone(), Event::SelFocus(source));
            }

            if key_focus && (!old_key_focus || target_is_new) {
                self.send_event(widget.re(), id.clone(), Event::KeyFocus);
            }

            if ime.is_some() && (ime != self.ime || target_is_new) {
                window.set_ime_allowed(ime);
                self.ime = ime;
            }
        }

        self.sel_focus = target;
    }

    /// Set navigation focus immediately
    fn set_nav_focus_impl(&mut self, mut widget: Node, target: Option<Id>, source: FocusSource) {
        if target == self.nav_focus || !self.config.nav_focus {
            return;
        }

        self.clear_key_focus();

        if let Some(old) = self.nav_focus.take() {
            self.action(&old, Action::REDRAW);
            self.send_event(widget.re(), old, Event::LostNavFocus);
        }

        self.nav_focus = target.clone();
        log::debug!(target: "kas_core::event", "nav_focus = {target:?}");
        if let Some(id) = target {
            self.action(&id, Action::REDRAW);
            self.send_event(widget, id, Event::NavFocus(source));
        }
    }

    /// Advance the keyboard navigation focus immediately
    fn next_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<Id>,
        reverse: bool,
        source: FocusSource,
    ) {
        if !self.config.nav_focus || (target.is_some() && target == self.nav_focus) {
            return;
        }

        if let Some(id) = self
            .popups
            .last()
            .filter(|popup| popup.is_sized)
            .map(|state| state.desc.id.clone())
        {
            if id.is_ancestor_of(widget.id_ref()) {
                // do nothing
            } else if let Some(r) = widget.find_node(&id, |node| {
                self.next_nav_focus_impl(node, target, reverse, source)
            }) {
                return r;
            } else {
                log::warn!(
                    target: "kas_core::event",
                    "next_nav_focus: have open pop-up which is not a child of widget",
                );
                return;
            }
        }

        let advance = if !reverse {
            NavAdvance::Forward(target.is_some())
        } else {
            NavAdvance::Reverse(target.is_some())
        };
        let focus = target.or_else(|| self.nav_focus.clone());

        // Whether to restart from the beginning on failure
        let restart = focus.is_some();

        let mut opt_id = self.nav_next(widget.re(), focus.as_ref(), advance);
        if restart && opt_id.is_none() {
            opt_id = self.nav_next(widget.re(), None, advance);
        }

        self.set_nav_focus_impl(widget, opt_id, source);
    }
}
