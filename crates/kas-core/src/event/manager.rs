// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

// Without winit, several things go unused
#![cfg_attr(not(feature = "winit"), allow(unused))]

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
use crate::cast::Cast;
use crate::geom::{Coord, Offset};
use crate::shell::ShellWindow;
use crate::util::WidgetHierarchy;
use crate::LayoutExt;
use crate::{Action, Erased, ErasedStack, NavAdvance, Node, Widget, WidgetId, WindowId};

mod config_mgr;
mod mgr_pub;
mod mgr_shell;
mod press;
pub use config_mgr::ConfigMgr;
pub use press::{GrabBuilder, Press, PressSource};

/// Controls the types of events delivered by [`Press::grab`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrabMode {
    /// Deliver [`Event::PressEnd`] only for each grabbed press
    Click,
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each grabbed press
    Grab,
    /// Deliver [`Event::Pan`] events, with scaling and rotation
    PanFull,
    /// Deliver [`Event::Pan`] events, with scaling
    PanScale,
    /// Deliver [`Event::Pan`] events, with rotation
    PanRotate,
    /// Deliver [`Event::Pan`] events, without scaling or rotation
    PanOnly,
}

impl GrabMode {
    fn is_pan(self) -> bool {
        use GrabMode::*;
        matches!(self, PanFull | PanScale | PanRotate | PanOnly)
    }
}

#[derive(Clone, Debug)]
struct MouseGrab {
    button: MouseButton,
    repetitions: u32,
    start_id: WidgetId,
    cur_id: Option<WidgetId>,
    depress: Option<WidgetId>,
    mode: GrabMode,
    pan_grab: (u16, u16),
    coord: Coord,
    delta: Offset,
}

impl<'a> EventMgr<'a> {
    fn flush_mouse_grab_motion(&mut self, widget: Node<'_>) {
        if let Some(grab) = self.mouse_grab.as_mut() {
            let delta = grab.delta;
            if delta == Offset::ZERO {
                return;
            }
            grab.delta = Offset::ZERO;

            match grab.mode {
                GrabMode::Click => {
                    if grab.start_id == grab.cur_id {
                        if grab.depress != grab.cur_id {
                            grab.depress = grab.cur_id.clone();
                            self.action |= Action::REDRAW;
                        }
                    } else {
                        if grab.depress.is_some() {
                            grab.depress = None;
                            self.action |= Action::REDRAW;
                        }
                    }
                }
                GrabMode::Grab => {
                    let target = grab.start_id.clone();
                    let press = Press {
                        source: PressSource::Mouse(grab.button, grab.repetitions),
                        id: grab.cur_id.clone(),
                        coord: grab.coord,
                    };
                    let event = Event::PressMove { press, delta };
                    self.send_event(widget, target, event);
                }
                _ => (),
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TouchGrab {
    id: u64,
    start_id: WidgetId,
    depress: Option<WidgetId>,
    cur_id: Option<WidgetId>,
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
            } else {
                if self.depress.is_some() {
                    self.depress = None;
                    return Action::REDRAW;
                }
            }
        }
        Action::empty()
    }

    fn flush_grab_move(&mut self) -> Option<(WidgetId, Event)> {
        if self.mode == GrabMode::Grab && self.last_move != self.coord {
            let delta = self.coord - self.last_move;
            let target = self.start_id.clone();
            let press = Press {
                source: PressSource::Touch(self.id),
                id: self.cur_id.clone(),
                coord: self.coord,
            };
            let event = Event::PressMove { press, delta };
            self.last_move = self.coord;
            Some((target, event))
        } else {
            None
        }
    }
}

const MAX_PAN_GRABS: usize = 2;

#[derive(Clone, Debug)]
struct PanGrab {
    id: WidgetId,
    mode: GrabMode,
    source_is_touch: bool,
    n: u16,
    coords: [(Coord, Coord); MAX_PAN_GRABS],
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)] // they all happen to be about Focus
enum Pending {
    Configure(WidgetId),
    Update(WidgetId),
    Send(WidgetId, Event),
    SetRect(WidgetId),
    NextNavFocus {
        target: Option<WidgetId>,
        reverse: bool,
        key_focus: bool,
    },
}

type AccelLayer = (bool, HashMap<VirtualKeyCode, WidgetId>);

/// Event manager state
///
/// This struct encapsulates window-specific event-handling state and handling.
/// Most operations are only available via a [`EventMgr`] handle, though some
/// are available on this struct.
///
/// Besides event handling, this struct also configures widgets.
///
/// Some methods are intended only for usage by KAS shells and are hidden from
/// documentation unless the `internal_doc` feature is enabled. Only [winit]
/// events are currently supported; changes will be required to generalise this.
///
/// [winit]: https://github.com/rust-windowing/winit
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
pub struct EventState {
    config: WindowConfig,
    disabled: Vec<WidgetId>,
    window_has_focus: bool,
    modifiers: ModifiersState,
    /// char focus is on same widget as sel_focus; otherwise its value is ignored
    char_focus: bool,
    sel_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    nav_fallback: Option<WidgetId>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_depress: LinearMap<u32, WidgetId>,
    last_mouse_coord: Coord,
    last_click_button: MouseButton,
    last_click_repetitions: u32,
    last_click_timeout: Instant,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 8]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    accel_layers: BTreeMap<WidgetId, AccelLayer>,
    // For each: (WindowId of popup, popup descriptor, old nav focus)
    popups: SmallVec<[(WindowId, crate::Popup, Option<WidgetId>); 16]>,
    popup_removed: SmallVec<[(WidgetId, WindowId); 16]>,
    time_updates: Vec<(Instant, WidgetId, u64)>,
    // Set of futures of messages together with id of sending widget
    fut_messages: Vec<(WidgetId, Pin<Box<dyn Future<Output = Erased>>>)>,
    // FIFO queue of events pending handling
    pending: VecDeque<Pending>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub action: Action,
}

/// internals
impl EventState {
    #[inline]
    fn char_focus(&self) -> Option<WidgetId> {
        if self.char_focus {
            self.sel_focus.clone()
        } else {
            None
        }
    }

    fn set_pan_on(
        &mut self,
        id: WidgetId,
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
            let p0 = grab.pan_grab.0;
            if usize::from(p0) >= index && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
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

    fn add_key_depress(&mut self, scancode: u32, id: WidgetId) {
        if self.key_depress.values().any(|v| *v == id) {
            return;
        }

        self.key_depress.insert(scancode, id);
        self.send_action(Action::REDRAW);
    }

    fn end_key_event(&mut self, scancode: u32) {
        // We must match scancode not vkey since the latter may have changed due to modifiers
        if let Some(id) = self.key_depress.remove(&scancode) {
            self.redraw(id);
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
                self.send_action(Action::REDRAW); // redraw(..)
                self.remove_pan_grab(grab.pan_grab);
                return Some(grab);
            }
        }
        None
    }

    fn clear_char_focus(&mut self) {
        if let Some(id) = self.char_focus() {
            log::trace!("clear_char_focus");
            // If widget has char focus, this is lost
            self.char_focus = false;
            self.pending
                .push_back(Pending::Send(id, Event::LostCharFocus));
        }
    }

    // Set selection focus to `wid`; if `char_focus` also set that
    fn set_sel_focus(&mut self, wid: WidgetId, char_focus: bool) {
        log::trace!("set_sel_focus: wid={wid}, char_focus={char_focus}");
        // The widget probably already has nav focus, but anyway:
        self.set_nav_focus(wid.clone(), true);

        if wid == self.sel_focus {
            self.char_focus = self.char_focus || char_focus;
            return;
        }

        if let Some(id) = self.sel_focus.clone() {
            if self.char_focus {
                // If widget has char focus, this is lost
                self.pending
                    .push_back(Pending::Send(id.clone(), Event::LostCharFocus));
            }

            // Selection focus is lost if another widget receives char focus
            self.pending
                .push_back(Pending::Send(id, Event::LostSelFocus));
        }

        self.char_focus = char_focus;
        self.sel_focus = Some(wid);
    }

    fn set_hover(&mut self, w_id: Option<WidgetId>) {
        if self.hover != w_id {
            log::trace!("set_hover: w_id={w_id:?}");
            if let Some(id) = self.hover.take() {
                self.pending
                    .push_back(Pending::Send(id, Event::LostMouseHover));
            }
            self.hover = w_id.clone();

            if let Some(id) = w_id {
                self.pending.push_back(Pending::Send(id, Event::MouseHover));
            }
        }
    }
}

/// Manager of event-handling and toolkit actions
///
/// `EventMgr` and [`EventState`] (available via [`Deref`]) support various
/// event management and event-handling state querying operations.
#[must_use]
pub struct EventMgr<'a> {
    state: &'a mut EventState,
    shell: &'a mut dyn ShellWindow,
    messages: &'a mut ErasedStack,
    last_child: Option<usize>,
    scroll: Scroll,
}

impl<'a> Deref for EventMgr<'a> {
    type Target = EventState;
    fn deref(&self) -> &Self::Target {
        self.state
    }
}
impl<'a> DerefMut for EventMgr<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

/// Internal methods
impl<'a> EventMgr<'a> {
    fn start_key_event(&mut self, mut widget: Node<'_>, vkey: VirtualKeyCode, scancode: u32) {
        log::trace!(
            "start_key_event: widget={}, vkey={vkey:?}, scancode={scancode}",
            widget.id()
        );

        use VirtualKeyCode as VK;

        let opt_command = self.config.shortcuts(|s| s.get(self.modifiers, vkey));

        if let Some(cmd) = opt_command {
            let mut targets = vec![];
            let mut send = |_self: &mut Self, id: WidgetId, cmd| -> bool {
                if !targets.contains(&id) {
                    let used = _self.send_event(widget.re(), id.clone(), Event::Command(cmd));
                    if used {
                        _self.add_key_depress(scancode, id.clone());
                    }
                    targets.push(id);
                    used
                } else {
                    false
                }
            };

            if self.char_focus || cmd.suitable_for_sel_focus() {
                if let Some(id) = self.sel_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if !self.modifiers.alt() {
                if let Some(id) = self.nav_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if let Some(id) = self.popups.last().map(|popup| popup.1.parent.clone()) {
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
                    if let Some(w) = widget.as_layout().get_id(id) {
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

        // Next priority goes to accelerator keys when Alt is held or alt_bypass is true
        let mut target = None;
        let mut n = 0;
        for (i, id) in (self.popups.iter().rev())
            .map(|(_, popup, _)| popup.parent.clone())
            .chain(std::iter::once(widget.id()))
            .enumerate()
        {
            if let Some(layer) = self.accel_layers.get(&id) {
                // but only when Alt is held or alt-bypass is enabled:
                if self.modifiers == ModifiersState::ALT
                    || layer.0 && self.modifiers == ModifiersState::empty()
                {
                    if let Some(id) = layer.1.get(&vkey).cloned() {
                        target = Some(id);
                        n = i;
                        break;
                    }
                }
            }
        }

        // If we found a key binding below the top layer, we should close everything above
        if n > 0 {
            let len = self.popups.len();
            for i in ((len - n)..len).rev() {
                let id = self.popups[i].0;
                self.close_window(id, false);
            }
        }

        if let Some(id) = target {
            if let Some(id) = widget._nav_next(self, Some(&id), NavAdvance::None) {
                self.set_nav_focus(id, true);
            }
            self.add_key_depress(scancode, id.clone());
            self.send_event(widget, id, Event::Command(Command::Activate));
        } else if self.config.nav_focus && vkey == VK::Tab {
            self.clear_char_focus();
            let shift = self.modifiers.shift();
            self.next_nav_focus_impl(widget.re(), None, shift, true);
        } else if vkey == VK::Escape {
            if let Some(id) = self.popups.last().map(|(id, _, _)| *id) {
                self.close_window(id, true);
            }
        }
    }

    // Clears mouse grab and pan grab, resets cursor and redraws
    fn remove_mouse_grab(&mut self, success: bool) -> Option<(WidgetId, Event)> {
        if let Some(grab) = self.mouse_grab.take() {
            log::trace!("remove_mouse_grab: start_id={}", grab.start_id);
            self.shell.set_cursor_icon(self.hover_icon);
            self.send_action(Action::REDRAW); // redraw(..)
            if grab.mode.is_pan() {
                self.remove_pan_grab(grab.pan_grab);
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
            panic!("steal_event affected EventMgr and returned Unused");
        }
    }

    pub(crate) fn post_send(&mut self, index: usize) -> Option<Scroll> {
        self.last_child = Some(index);
        (self.scroll != Scroll::None).then_some(self.scroll)
    }

    /// Replay a message as if it was pushed by `id`
    fn replay(&mut self, mut widget: Node<'_>, id: WidgetId, msg: Erased) {
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.messages.set_base();
        log::trace!(target: "kas_core::event::manager", "replay: id={id}: {msg:?}");

        widget._replay(self, id, msg);
        self.last_child = None;
        self.scroll = Scroll::None;
    }

    // Wrapper around Self::send; returns true when event is used
    #[inline]
    fn send_event(&mut self, widget: Node<'_>, id: WidgetId, event: Event) -> bool {
        self.messages.set_base();
        let used = self.send_event_impl(widget, id, event);
        self.last_child = None;
        self.scroll = Scroll::None;
        used
    }

    // Send an event; possibly leave messages on the stack
    fn send_event_impl(&mut self, mut widget: Node<'_>, mut id: WidgetId, event: Event) -> bool {
        debug_assert!(self.scroll == Scroll::None);
        debug_assert!(self.last_child.is_none());
        self.messages.set_base();
        log::trace!(target: "kas_core::event::manager", "send_event: id={id}: {event:?}");

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
                log::trace!(target: "kas_core::event::manager", "target is disabled; sending to ancestor {id}");
            }
        }

        widget._send(self, id, disabled, event) == Response::Used
    }

    // Returns true if event is used
    fn send_popup_first(
        &mut self,
        mut widget: Node<'_>,
        id: Option<WidgetId>,
        event: Event,
    ) -> bool {
        while let Some((wid, parent)) = self
            .popups
            .last()
            .map(|(wid, p, _)| (*wid, p.parent.clone()))
        {
            log::trace!("send_popup_first: parent={parent}: {event:?}");
            if self.send_event(widget.re(), parent, event.clone()) {
                return true;
            }
            self.close_window(wid, false);
        }
        if let Some(id) = id {
            return self.send_event(widget, id, event);
        }
        false
    }

    /// Advance the keyboard navigation focus
    pub fn next_nav_focus_impl(
        &mut self,
        mut widget: Node,
        target: Option<WidgetId>,
        reverse: bool,
        key_focus: bool,
    ) {
        if !self.config.nav_focus || (target.is_some() && target == self.nav_focus) {
            return;
        }

        if let Some(id) = self.popups.last().map(|(_, p, _)| p.id.clone()) {
            if id.is_ancestor_of(widget.id_ref()) {
                // do nothing
            } else if let Some(r) = widget.for_id(&id, |node| {
                self.next_nav_focus_impl(node, target, reverse, key_focus)
            }) {
                return r;
            } else {
                log::warn!(
                    target: "kas_core::event::config_mgr",
                    "next_nav_focus: have open pop-up which is not a child of widget",
                );
                return;
            }
        }

        // We redraw in all cases. Since this is not part of widget event
        // processing, we can push directly to self.action.
        self.send_action(Action::REDRAW);

        let advance = if !reverse {
            NavAdvance::Forward(target.is_some())
        } else {
            NavAdvance::Reverse(target.is_some())
        };
        let focus = target.or_else(|| self.nav_focus.clone());

        // Whether to restart from the beginning on failure
        let restart = focus.is_some();

        let mut opt_id = widget._nav_next(self, focus.as_ref(), advance);
        if restart && opt_id.is_none() {
            opt_id = widget._nav_next(self, None, advance);
        }

        log::trace!(
            target: "kas_core::event::config_mgr",
            "next_nav_focus: nav_focus={opt_id:?}",
        );
        if opt_id == self.nav_focus {
            return;
        }

        if let Some(id) = self.nav_focus.clone() {
            self.pending
                .push_back(Pending::Send(id, Event::LostNavFocus));
        }
        if self.sel_focus != opt_id {
            self.clear_char_focus();
        }

        self.nav_focus = opt_id.clone();
        if let Some(id) = opt_id {
            self.pending
                .push_back(Pending::Send(id, Event::NavFocus(key_focus)));
        } else {
            // Most likely an error occurred
        }
    }
}
