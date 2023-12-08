// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event context state

// Without winit, several things go unused
#![cfg_attr(not(winit), allow(unused))]

use linear_map::LinearMap;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::time::Instant;
use std::u16;

use super::config::WindowConfig;
use super::*;
use crate::app::{AppShared, Platform, WindowDataErased};
use crate::cast::Cast;
use crate::geom::Coord;
use crate::messages::{Erased, MessageStack};
use crate::util::WidgetHierarchy;
use crate::LayoutExt;
use crate::{Action, Id, NavAdvance, Node, Widget, WindowId};

mod config;
mod cx_pub;
mod platform;
mod press;

pub use config::ConfigCx;
pub use press::{GrabBuilder, Press, PressSource};

/// Controls the types of events delivered by [`Press::grab`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrabMode {
    /// Deliver [`Event::PressEnd`] only for each grabbed press
    Click,
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each grabbed press
    Grab,
    /// Deliver [`Event::Pan`] events, without scaling or rotation
    PanOnly,
    /// Deliver [`Event::Pan`] events, with rotation
    PanRotate,
    /// Deliver [`Event::Pan`] events, with scaling
    PanScale,
    /// Deliver [`Event::Pan`] events, with scaling and rotation
    PanFull,
}

impl GrabMode {
    /// True for "pan" variants
    pub fn is_pan(self) -> bool {
        use GrabMode::*;
        matches!(self, PanFull | PanScale | PanRotate | PanOnly)
    }
}

#[derive(Clone, Debug)]
enum GrabDetails {
    Click { cur_id: Option<Id> },
    Grab,
    Pan((u16, u16)),
}

impl GrabDetails {
    fn is_pan(&self) -> bool {
        matches!(self, GrabDetails::Pan(_))
    }
}

#[derive(Clone, Debug)]
struct MouseGrab {
    button: MouseButton,
    repetitions: u32,
    start_id: Id,
    depress: Option<Id>,
    details: GrabDetails,
}

impl<'a> EventCx<'a> {
    fn flush_mouse_grab_motion(&mut self) {
        if let Some(grab) = self.mouse_grab.as_mut() {
            match grab.details {
                GrabDetails::Click { ref cur_id } => {
                    if grab.start_id == cur_id {
                        if grab.depress != *cur_id {
                            grab.depress = cur_id.clone();
                            self.action |= Action::REDRAW;
                        }
                    } else if grab.depress.is_some() {
                        grab.depress = None;
                        self.action |= Action::REDRAW;
                    }
                }
                _ => (),
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TouchGrab {
    id: u64,
    start_id: Id,
    depress: Option<Id>,
    cur_id: Option<Id>,
    last_move: Coord,
    coord: Coord,
    mode: GrabMode,
    pan_grab: (u16, u16),
}

impl TouchGrab {
    fn flush_click_move(&mut self) -> Action {
        if self.mode == GrabMode::Click && self.last_move != self.coord {
            self.last_move = self.coord;
            if self.start_id == self.cur_id {
                if self.depress != self.cur_id {
                    self.depress = self.cur_id.clone();
                    return Action::REDRAW;
                }
            } else if self.depress.is_some() {
                self.depress = None;
                return Action::REDRAW;
            }
        }
        Action::empty()
    }
}

const MAX_PAN_GRABS: usize = 2;

#[derive(Clone, Debug)]
struct PanGrab {
    id: Id,
    mode: GrabMode,
    source_is_touch: bool,
    n: u16,
    coords: [(Coord, Coord); MAX_PAN_GRABS],
}

#[derive(Debug)]
struct PendingSelFocus {
    target: Option<Id>,
    key_focus: bool,
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
/// feature is enabled. Only [winit]
/// events are currently supported; changes will be required to generalise this.
///
/// [winit]: https://github.com/rust-windowing/winit
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
pub struct EventState {
    config: WindowConfig,
    platform: Platform,
    disabled: Vec<Id>,
    window_has_focus: bool,
    modifiers: ModifiersState,
    /// key focus is on same widget as sel_focus; otherwise its value is ignored
    key_focus: bool,
    sel_focus: Option<Id>,
    nav_focus: Option<Id>,
    nav_fallback: Option<Id>,
    hover: Option<Id>,
    hover_icon: CursorIcon,
    old_hover_icon: CursorIcon,
    key_depress: LinearMap<PhysicalKey, Id>,
    last_mouse_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 8]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    access_layers: BTreeMap<Id, AccessLayer>,
    // For each: (WindowId of popup, popup descriptor, old nav focus)
    popups: SmallVec<[(WindowId, crate::PopupDescriptor, Option<Id>); 16]>,
    popup_removed: SmallVec<[(Id, WindowId); 16]>,
    time_updates: Vec<(Instant, Id, u64)>,
    // Set of futures of messages together with id of sending widget
    fut_messages: Vec<(Id, Pin<Box<dyn Future<Output = Erased>>>)>,
    // Widget requiring update (and optionally configure)
    pending_update: Option<(Id, bool)>,
    // Optional new target for selection focus. bool is true if this also gains key focus.
    pending_sel_focus: Option<PendingSelFocus>,
    pending_nav_focus: PendingNavFocus,
    pending_cmds: VecDeque<(Id, Command)>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub action: Action,
}

/// internals
impl EventState {
    #[inline]
    fn key_focus(&self) -> Option<Id> {
        if self.key_focus {
            self.sel_focus.clone()
        } else {
            None
        }
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
                    source: FocusSource::Synthetic,
                });
            }
        }
    }

    fn set_pan_on(
        &mut self,
        id: Id,
        mode: GrabMode,
        source_is_touch: bool,
        coord: Coord,
    ) -> (u16, u16) {
        for (gi, grab) in self.pan_grab.iter_mut().enumerate() {
            if grab.id == id {
                if grab.source_is_touch != source_is_touch {
                    self.remove_pan(gi);
                    break;
                }

                debug_assert_eq!(grab.mode, mode);

                let index = grab.n;
                if usize::from(index) < MAX_PAN_GRABS {
                    grab.coords[usize::from(index)] = (coord, coord);
                }
                grab.n = index + 1;
                return (gi.cast(), index);
            }
        }

        let gj = self.pan_grab.len().cast();
        let n = 1;
        let mut coords: [(Coord, Coord); MAX_PAN_GRABS] = Default::default();
        coords[0] = (coord, coord);
        log::trace!("set_pan_on: index={}, id={id}", self.pan_grab.len());
        self.pan_grab.push(PanGrab {
            id,
            mode,
            source_is_touch,
            n,
            coords,
        });
        (gj, 0)
    }

    fn remove_pan(&mut self, index: usize) {
        log::trace!("remove_pan: index={index}");
        self.pan_grab.remove(index);
        if let Some(grab) = &mut self.mouse_grab {
            if let GrabDetails::Pan(ref mut g) = grab.details {
                if usize::from(g.0) >= index {
                    g.0 -= 1;
                }
            }
        }
        for grab in self.touch_grab.iter_mut() {
            let p0 = grab.pan_grab.0;
            if usize::from(p0) >= index && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
    }

    fn remove_pan_grab(&mut self, g: (u16, u16)) {
        if let Some(grab) = self.pan_grab.get_mut(usize::from(g.0)) {
            grab.n -= 1;
            if grab.n == 0 {
                return self.remove_pan(g.0.into());
            }
            assert!(grab.source_is_touch);
            for i in (usize::from(g.1))..(usize::from(grab.n) - 1) {
                grab.coords[i] = grab.coords[i + 1];
            }
        } else {
            return;
        }

        // Note: the fact that grab.n > 0 implies source is a touch event!
        for grab in self.touch_grab.iter_mut() {
            if grab.pan_grab.0 == g.0 && grab.pan_grab.1 > g.1 {
                grab.pan_grab.1 -= 1;
                if usize::from(grab.pan_grab.1) == MAX_PAN_GRABS - 1 {
                    let v = grab.coord;
                    self.pan_grab[usize::from(g.0)].coords[usize::from(grab.pan_grab.1)] = (v, v);
                }
            }
        }
    }

    #[inline]
    fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchGrab> {
        self.touch_grab.iter_mut().find(|grab| grab.id == touch_id)
    }

    // Clears touch grab and pan grab and redraws
    fn remove_touch(&mut self, touch_id: u64) -> Option<TouchGrab> {
        for i in 0..self.touch_grab.len() {
            if self.touch_grab[i].id == touch_id {
                let grab = self.touch_grab.remove(i);
                log::trace!(
                    "remove_touch: touch_id={touch_id}, start_id={}",
                    grab.start_id
                );
                self.opt_action(grab.depress.clone(), Action::REDRAW);
                self.remove_pan_grab(grab.pan_grab);
                return Some(grab);
            }
        }
        None
    }
}

/// Event handling context
///
/// `EventCx` and [`EventState`] (available via [`Deref`]) support various
/// event management and event-handling state querying operations.
#[must_use]
pub struct EventCx<'a> {
    state: &'a mut EventState,
    shared: &'a mut dyn AppShared,
    window: &'a dyn WindowDataErased,
    messages: &'a mut MessageStack,
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

/// Internal methods
impl<'a> EventCx<'a> {
    fn start_key_event(&mut self, mut widget: Node<'_>, vkey: Key, code: PhysicalKey) {
        log::trace!(
            "start_key_event: widget={}, vkey={vkey:?}, physical_key={code:?}",
            widget.id()
        );

        let opt_command = self
            .config
            .shortcuts(|s| s.try_match(self.modifiers, &vkey));

        if let Some(cmd) = opt_command {
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

            if self.key_focus || cmd.suitable_for_sel_focus() {
                if let Some(id) = self.sel_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if !self.modifiers.alt_key() {
                if let Some(id) = self.nav_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if let Some(id) = self.popups.last().map(|popup| popup.1.id.clone()) {
                if send(self, id, cmd) {
                    return;
                }
            }

            if let Some(id) = self.nav_fallback.clone() {
                if send(self, id, cmd) {
                    return;
                }
            }

            if matches!(cmd, Command::Debug) {
                if let Some(ref id) = self.hover {
                    if let Some(w) = widget.as_layout().find_widget(id) {
                        let hier = WidgetHierarchy::new(w);
                        log::debug!("Widget heirarchy (from mouse): {hier}");
                    }
                } else {
                    let hier = WidgetHierarchy::new(widget.as_layout());
                    log::debug!("Widget heirarchy (whole window): {hier}");
                }
                return;
            }
        }

        // Next priority goes to access keys when Alt is held or alt_bypass is true
        let mut target = None;
        for id in (self.popups.iter().rev())
            .map(|(_, popup, _)| popup.id.clone())
            .chain(std::iter::once(widget.id()))
        {
            if let Some(layer) = self.access_layers.get(&id) {
                // but only when Alt is held or alt-bypass is enabled:
                if self.modifiers == ModifiersState::ALT
                    || layer.0 && self.modifiers == ModifiersState::empty()
                {
                    if let Some(id) = layer.1.get(&vkey).cloned() {
                        target = Some(id);
                        break;
                    }
                }
            }
        }

        if let Some(id) = target {
            if let Some(id) = self.nav_next(widget.re(), Some(&id), NavAdvance::None) {
                self.set_nav_focus(id, FocusSource::Key);
            }
            let event = Event::Command(Command::Activate, Some(code));
            self.send_event(widget, id, event);
        } else if self.config.nav_focus && vkey == Key::Named(NamedKey::Tab) {
            let shift = self.modifiers.shift_key();
            self.next_nav_focus_impl(widget.re(), None, shift, FocusSource::Key);
        } else if vkey == Key::Named(NamedKey::Escape) {
            if let Some(id) = self.popups.last().map(|(id, _, _)| *id) {
                self.close_window(id);
            }
        }
    }

    // Clears mouse grab and pan grab, resets cursor and redraws
    fn remove_mouse_grab(&mut self, success: bool) -> Option<(Id, Event)> {
        if let Some(grab) = self.mouse_grab.take() {
            log::trace!("remove_mouse_grab: start_id={}", grab.start_id);
            self.window.set_cursor_icon(self.hover_icon);
            self.opt_action(grab.depress.clone(), Action::REDRAW);
            if let GrabDetails::Pan(g) = grab.details {
                self.remove_pan_grab(g);
                // Pan grabs do not receive Event::PressEnd
                None
            } else {
                let press = Press {
                    source: PressSource::Mouse(grab.button, grab.repetitions),
                    id: self.hover.clone(),
                    coord: self.last_mouse_coord,
                };
                let event = Event::PressEnd { press, success };
                Some((grab.start_id, event))
            }
        } else {
            None
        }
    }

    pub(crate) fn assert_post_steal_unused(&self) {
        if self.scroll != Scroll::None || self.messages.has_any() {
            panic!("steal_event affected EventCx and returned Unused");
        }
    }

    pub(crate) fn post_send(&mut self, index: usize) -> Option<Scroll> {
        self.last_child = Some(index);
        (self.scroll != Scroll::None).then_some(self.scroll)
    }

    /// Replay a message as if it was pushed by `id`
    fn replay(&mut self, mut widget: Node<'_>, id: Id, msg: Erased) {
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.messages.set_base();
        log::trace!(target: "kas_core::event", "replay: id={id}: {msg:?}");

        widget._replay(self, id, msg);
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

        let used = widget._send(self, id, disabled, event) == Used;

        self.last_child = None;
        self.scroll = Scroll::None;
        used
    }

    fn send_popup_first(&mut self, mut widget: Node<'_>, id: Option<Id>, event: Event) {
        while let Some(pid) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
            let mut target = pid;
            if let Some(id) = id.clone() {
                if target.is_ancestor_of(&id) {
                    target = id;
                }
            }
            log::trace!("send_popup_first: id={target}: {event:?}");
            if self.send_event(widget.re(), target, event.clone()) {
                return;
            }
        }
        if let Some(id) = id {
            self.send_event(widget, id, event);
        }
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

    // Clear old hover, set new hover, send events.
    // If there is a popup, only permit descendands of that.
    fn set_hover(&mut self, mut widget: Node<'_>, mut w_id: Option<Id>) {
        if let Some(ref id) = w_id {
            if let Some(popup) = self.popups.last() {
                if !popup.1.id.is_ancestor_of(id) {
                    w_id = None;
                }
            }
        }

        if self.hover != w_id {
            log::trace!("set_hover: w_id={w_id:?}");
            self.hover_icon = Default::default();
            if let Some(id) = self.hover.take() {
                self.send_event(widget.re(), id, Event::MouseHover(false));
            }
            self.hover = w_id.clone();

            if let Some(id) = w_id {
                self.send_event(widget, id, Event::MouseHover(true));
            }
        }
    }

    // Set selection focus to `wid` immediately; if `key_focus` also set that
    fn set_sel_focus(&mut self, mut widget: Node<'_>, pending: PendingSelFocus) {
        let PendingSelFocus {
            target,
            key_focus,
            source,
        } = pending;

        log::trace!("set_sel_focus: target={target:?}, key_focus={key_focus}");

        if target == self.sel_focus {
            self.key_focus = target.is_some() && (self.key_focus || key_focus);
            return;
        }

        if let Some(id) = self.sel_focus.clone() {
            if self.key_focus {
                // If widget has key focus, this is lost
                self.send_event(widget.re(), id.clone(), Event::LostKeyFocus);
            }

            // Selection focus is lost if another widget receives key focus
            self.send_event(widget.re(), id, Event::LostSelFocus);
        }

        self.key_focus = key_focus;
        self.sel_focus = target.clone();

        if let Some(id) = target {
            // The widget probably already has nav focus, but anyway:
            self.set_nav_focus(id.clone(), FocusSource::Synthetic);

            self.send_event(widget.re(), id.clone(), Event::SelFocus(source));
            if key_focus {
                self.send_event(widget, id, Event::KeyFocus);
            }
        }
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

        if let Some(id) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
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
