// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

// Without winit, several things go unused
#![cfg_attr(not(feature = "winit"), allow(unused))]

use linear_map::LinearMap;
use log::{trace, warn};
use smallvec::SmallVec;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::time::Instant;
use std::u16;

use super::config::WindowConfig;
use super::*;
use crate::cast::Cast;
use crate::geom::{Coord, Offset};
use crate::{ShellWindow, TkAction, Widget, WidgetExt, WidgetId, WindowId};

mod mgr_pub;
mod mgr_shell;

/// Controls the types of events delivered by [`EventMgr::grab_press`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrabMode {
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

impl MouseGrab {
    fn flush_move(&mut self) -> Option<(WidgetId, Event)> {
        if self.delta != Offset::ZERO {
            let event = Event::PressMove {
                source: PressSource::Mouse(self.button, self.repetitions),
                cur_id: self.cur_id.clone(),
                coord: self.coord,
                delta: self.delta,
            };
            self.delta = Offset::ZERO;
            Some((self.start_id.clone(), event))
        } else {
            None
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
    fn flush_move(&mut self) -> Option<(WidgetId, Event)> {
        if self.last_move != self.coord {
            let delta = self.coord - self.last_move;
            let target = self.start_id.clone();
            let event = Event::PressMove {
                source: PressSource::Touch(self.id),
                cur_id: self.cur_id.clone(),
                coord: self.coord,
                delta,
            };
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
    LostCharFocus(WidgetId),
    LostSelFocus(WidgetId),
    SetNavFocus(WidgetId, bool),
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
#[derive(Debug)]
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
    hover_highlight: bool,
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
    pending: SmallVec<[Pending; 8]>,
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub action: TkAction,
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
        trace!("EventMgr: start pan grab {} on {}", self.pan_grab.len(), id);
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
        trace!("EventMgr: end pan grab {}", index);
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
        self.send_action(TkAction::REDRAW);
    }

    fn end_key_event(&mut self, scancode: u32) {
        // We must match scancode not vkey since the latter may have changed due to modifiers
        if let Some(id) = self.key_depress.remove(&scancode) {
            self.redraw(id);
        }
    }

    fn mouse_grab(&mut self) -> Option<&mut MouseGrab> {
        self.mouse_grab.as_mut()
    }

    #[inline]
    fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchGrab> {
        for grab in self.touch_grab.iter_mut() {
            if grab.id == touch_id {
                return Some(grab);
            }
        }
        None
    }

    // Clears touch grab and pan grab and redraws
    fn remove_touch(&mut self, touch_id: u64) -> Option<TouchGrab> {
        for i in 0..self.touch_grab.len() {
            if self.touch_grab[i].id == touch_id {
                let grab = self.touch_grab.remove(i);
                trace!("EventMgr: end touch grab by {}", grab.start_id);
                self.send_action(TkAction::REDRAW); // redraw(..)
                self.remove_pan_grab(grab.pan_grab);
                return Some(grab);
            }
        }
        None
    }

    fn clear_char_focus(&mut self) {
        trace!("EventMgr::clear_char_focus");
        if let Some(id) = self.char_focus() {
            // If widget has char focus, this is lost
            self.char_focus = false;
            self.pending.push(Pending::LostCharFocus(id));
        }
    }

    // Set selection focus to `wid`; if `char_focus` also set that
    fn set_sel_focus(&mut self, wid: WidgetId, char_focus: bool) {
        trace!(
            "EventMgr::set_sel_focus: wid={}, char_focus={}",
            wid,
            char_focus
        );
        // The widget probably already has nav focus, but anyway:
        self.set_nav_focus(wid.clone(), true);

        if wid == self.sel_focus {
            self.char_focus = self.char_focus || char_focus;
            return;
        }

        if let Some(id) = self.sel_focus.clone() {
            if self.char_focus {
                // If widget has char focus, this is lost
                self.pending.push(Pending::LostCharFocus(id.clone()));
            }

            // Selection focus is lost if another widget receives char focus
            self.pending.push(Pending::LostSelFocus(id));
        }

        self.char_focus = char_focus;
        self.sel_focus = Some(wid);
    }
}

// NOTE: we *want* to store Box<dyn Any + Debug> entries, but Rust doesn't
// support multi-trait objects. An alternative would be to store Box<dyn Message>
// where `trait Message: Any + Debug {}`, but Rust does not support
// trait-object upcast, so we cannot downcast the result.
//
// Workaround: pre-format when the message is *pushed*.
struct Message {
    any: Box<dyn Any>,
    #[cfg(debug_assertions)]
    fmt: String,
}
impl Message {
    fn new<M: Any + Debug>(msg: Box<M>) -> Self {
        #[cfg(debug_assertions)]
        let fmt = format!("{:?}", &msg);
        let any = msg;
        Message {
            #[cfg(debug_assertions)]
            fmt,
            any,
        }
    }

    fn is<T: 'static>(&self) -> bool {
        self.any.is::<T>()
    }

    fn downcast<T: 'static>(self) -> Result<Box<T>, Box<dyn Any>> {
        self.any.downcast::<T>()
    }
}

/// Manager of event-handling and toolkit actions
///
/// An `EventMgr` is in fact a handle around [`EventState`] and [`ShellWindow`]
/// in order to provide a convenient user-interface during event processing.
///
/// `EventMgr` supports [`Deref`] and [`DerefMut`] with target [`EventState`].
///
/// It exposes two interfaces: one aimed at users implementing widgets and UIs
/// and one aimed at shells. The latter is hidden
/// from documentation unless the `internal_doc` feature is enabled.
#[must_use]
pub struct EventMgr<'a> {
    state: &'a mut EventState,
    shell: &'a mut dyn ShellWindow,
    messages: Vec<Message>,
    scroll: Scroll,
    action: TkAction,
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

impl<'a> Drop for EventMgr<'a> {
    fn drop(&mut self) {
        for _msg in self.messages.drain(..) {
            #[cfg(debug_assertions)]
            log::warn!("EventMgr: unhandled message: {}", _msg.fmt);
            #[cfg(not(debug_assertions))]
            log::warn!("EventMgr: unhandled message: [use debug build to see value]");
        }
    }
}

/// Internal methods
impl<'a> EventMgr<'a> {
    fn set_hover(&mut self, widget: &dyn Widget, w_id: Option<WidgetId>) {
        if self.state.hover != w_id {
            trace!("EventMgr: hover = {:?}", w_id);
            if let Some(id) = self.state.hover.take() {
                if self.state.hover_highlight {
                    self.redraw(id);
                }
            }
            self.state.hover = w_id.clone();

            if let Some(id) = w_id {
                let mut icon = Default::default();
                if !self.is_disabled(&id) {
                    if let Some(w) = widget.find_widget(&id) {
                        self.state.hover_highlight = w.hover_highlight();
                        if self.state.hover_highlight {
                            self.redraw(id);
                        }
                        icon = w.cursor_icon();
                    }
                }
                if icon != self.state.hover_icon {
                    self.state.hover_icon = icon;
                    if self.state.mouse_grab.is_none() {
                        self.shell.set_cursor_icon(icon);
                    }
                }
            }
        }
    }

    fn start_key_event(&mut self, widget: &mut dyn Widget, vkey: VirtualKeyCode, scancode: u32) {
        trace!(
            "EventMgr::start_key_event: widget={}, vkey={:?}, scancode={}",
            widget.id(),
            vkey,
            scancode
        );

        use VirtualKeyCode as VK;

        let opt_command = self
            .state
            .config
            .shortcuts(|s| s.get(self.state.modifiers, vkey));

        if let Some(cmd) = opt_command {
            let mut targets = vec![];
            let mut send = |_self: &mut Self, id: WidgetId, cmd| -> bool {
                if !targets.contains(&id) {
                    let used = _self.send_event(widget, id.clone(), Event::Command(cmd));
                    if used {
                        _self.add_key_depress(scancode, id.clone());
                    }
                    targets.push(id);
                    used
                } else {
                    false
                }
            };

            if self.state.char_focus {
                if let Some(id) = self.state.sel_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if !self.state.modifiers.alt() {
                if let Some(id) = self.state.nav_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if let Some(id) = self.state.popups.last().map(|popup| popup.1.parent.clone()) {
                if send(self, id, cmd) {
                    return;
                }
            }

            if cmd.suitable_for_sel_focus() {
                if let Some(id) = self.state.sel_focus.clone() {
                    if send(self, id, cmd) {
                        return;
                    }
                }
            }

            if let Some(id) = self.state.nav_fallback.clone() {
                if send(self, id, cmd) {
                    return;
                }
            }
        }

        // Next priority goes to accelerator keys when Alt is held or alt_bypass is true
        let mut target = None;
        let mut n = 0;
        for (i, id) in (self.state.popups.iter().rev())
            .map(|(_, popup, _)| popup.parent.clone())
            .chain(std::iter::once(widget.id()))
            .enumerate()
        {
            if let Some(layer) = self.state.accel_layers.get(&id) {
                // but only when Alt is held or alt-bypass is enabled:
                if self.state.modifiers.alt() || layer.0 {
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
            let len = self.state.popups.len();
            for i in ((len - n)..len).rev() {
                let id = self.state.popups[i].0;
                self.close_window(id, false);
            }
        }

        if let Some(id) = target {
            if widget
                .find_widget(&id)
                .map(|w| w.key_nav())
                .unwrap_or(false)
            {
                self.set_nav_focus(id.clone(), true);
            }
            self.add_key_depress(scancode, id.clone());
            self.send_event(widget, id, Event::Command(Command::Activate));
        } else if vkey == VK::Tab {
            self.clear_char_focus();
            let shift = self.state.modifiers.shift();
            self.next_nav_focus(widget, shift, true);
        } else if vkey == VK::Escape {
            if let Some(id) = self.state.popups.last().map(|(id, _, _)| *id) {
                self.close_window(id, true);
            }
        }
    }

    // Clears mouse grab and pan grab, resets cursor and redraws
    fn remove_mouse_grab(&mut self) -> Option<MouseGrab> {
        if let Some(grab) = self.state.mouse_grab.take() {
            trace!("EventMgr: end mouse grab by {}", grab.start_id);
            self.shell.set_cursor_icon(self.state.hover_icon);
            self.send_action(TkAction::REDRAW); // redraw(..)
            self.state.remove_pan_grab(grab.pan_grab);
            Some(grab)
        } else {
            None
        }
    }

    // Traverse widget tree by recursive call to a specific target
    //
    // Note: cannot use internal stack of mutable references due to borrow checker
    fn send_recurse(
        &mut self,
        widget: &mut dyn Widget,
        id: WidgetId,
        disabled: bool,
        event: Event,
    ) -> Response {
        let mut response = Response::Unused;
        if widget.steal_event(self, &id, &event) == Response::Used {
            response = Response::Used;
        } else if let Some(index) = widget.find_child_index(&id) {
            let translation = widget.translation();
            if let Some(w) = widget.get_child_mut(index) {
                response = self.send_recurse(w, id, disabled, event.clone() + translation);
                if self.scroll != Scroll::None {
                    widget.handle_scroll(self, self.scroll);
                }
            } else {
                warn!(
                    "Widget {} found index {index} for {id}, but child not found",
                    widget.identify()
                );
            }

            if matches!(response, Response::Unused) {
                response = widget.handle_unused(self, index, event);
            } else if self.has_msg() {
                widget.handle_message(self, index);
            }
        } else if disabled {
            // event is unused
        } else if id == widget.id_ref() {
            if event == Event::NavFocus(true) {
                self.set_scroll(Scroll::Rect(widget.rect()));
                response = Response::Used;
            }

            response |= widget.handle_event(self, event)
        } else {
            warn!("Widget {} cannot find path to {id}", widget.identify());
        }

        response
    }

    // Traverse widget tree by recursive call, broadcasting
    fn send_all(&mut self, widget: &mut dyn Widget, event: Event) -> usize {
        let child_event = event.clone() + widget.translation();
        widget.handle_event(self, event);
        let mut count = 1;
        for index in 0..widget.num_children() {
            if let Some(w) = widget.get_child_mut(index) {
                count += self.send_all(w, child_event.clone());
            }
        }
        count
    }

    // Wrapper around Self::send; returns true when event is used
    #[inline]
    fn send_event(&mut self, widget: &mut dyn Widget, id: WidgetId, event: Event) -> bool {
        self.send(widget, id, event) == Response::Used
    }

    fn send_popup_first(&mut self, widget: &mut dyn Widget, id: Option<WidgetId>, event: Event) {
        while let Some((wid, parent)) = self
            .state
            .popups
            .last()
            .map(|(wid, p, _)| (*wid, p.parent.clone()))
        {
            trace!("Send to popup parent: {}: {:?}", parent, event);
            match self.send(widget, parent, event.clone()) {
                Response::Unused => (),
                _ => return,
            }
            self.close_window(wid, false);
        }
        if let Some(id) = id {
            self.send_event(widget, id, event);
        }
    }
}
