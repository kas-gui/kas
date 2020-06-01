// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

// Without winit, several things go unused
#![cfg_attr(not(feature = "winit"), allow(unused))]

use log::trace;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::Instant;
use std::u16;

use super::*;
use crate::geom::Coord;
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{TkAction, TkWindow, Widget, WidgetId, WindowId};

mod mgr_pub;
mod mgr_tk;

/// Controls the types of events delivered by [`Manager::request_grab`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrabMode {
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each press
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
    start_id: WidgetId,
    depress: Option<WidgetId>,
    mode: GrabMode,
    pan_grab: (u16, u16),
}

#[derive(Clone, Debug)]
struct TouchGrab {
    touch_id: u64,
    start_id: WidgetId,
    depress: Option<WidgetId>,
    cur_id: Option<WidgetId>,
    coord: Coord,
    mode: GrabMode,
    pan_grab: (u16, u16),
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
enum Pending {
    LostCharFocus(WidgetId),
}

/// Event manager state
///
/// This struct encapsulates window-specific event-handling state and handling.
/// Most operations are only available via a [`Manager`] handle, though some
/// are available on this struct.
///
/// Besides event handling, this struct also configures widgets.
///
/// Some methods are intended only for toolkit usage and are hidden from
/// documentation unless the `internal_doc` feature is enabled.
//
// Note that the most frequent usage of fields is to check highlighting states
// for each widget during drawing. Most fields contain only a few values, hence
// `SmallVec` is used to keep contents in local memory.
#[derive(Debug)]
pub struct ManagerState {
    end_id: WidgetId,
    dpi_factor: f64,
    modifiers: ModifiersState,
    char_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    nav_fallback: Option<WidgetId>,
    nav_stack: SmallVec<[u32; 16]>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_depress: SmallVec<[(u32, WidgetId); 10]>,
    last_mouse_coord: Coord,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 10]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    accel_stack: Vec<(bool, HashMap<VirtualKeyCode, WidgetId>)>,
    accel_layers: HashMap<WidgetId, (bool, HashMap<VirtualKeyCode, WidgetId>)>,
    popups: SmallVec<[(WindowId, kas::Popup); 16]>,
    new_popups: SmallVec<[WidgetId; 16]>,
    popup_removed: SmallVec<[(WidgetId, WindowId); 16]>,

    time_start: Instant,
    time_updates: Vec<(Instant, WidgetId)>,
    // TODO(opt): consider other containers, e.g. C++ multimap
    // or sorted Vec with binary search yielding a range
    handle_updates: HashMap<UpdateHandle, Vec<WidgetId>>,
    pending: SmallVec<[Pending; 8]>,
    action: TkAction,
}

/// internals
impl ManagerState {
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
                if (index as usize) < MAX_PAN_GRABS {
                    grab.coords[index as usize] = (coord, coord);
                }
                grab.n = index + 1;
                return (gi as u16, index);
            }
        }

        let gj = self.pan_grab.len() as u16;
        let n = 1;
        let mut coords: [(Coord, Coord); MAX_PAN_GRABS] = Default::default();
        coords[0] = (coord, coord);
        trace!("Manager: start pan grab {} on {}", self.pan_grab.len(), id);
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
        trace!("Manager: end pan grab {}", index);
        self.pan_grab.remove(index);
        if let Some(grab) = &mut self.mouse_grab {
            let p0 = grab.pan_grab.0;
            if p0 >= index as u16 && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
        for grab in &mut self.touch_grab {
            let p0 = grab.pan_grab.0;
            if p0 >= index as u16 && p0 != u16::MAX {
                grab.pan_grab.0 = p0 - 1;
            }
        }
    }

    fn remove_pan_grab(&mut self, g: (u16, u16)) {
        if let Some(grab) = self.pan_grab.get_mut(g.0 as usize) {
            grab.n -= 1;
            if grab.n == 0 {
                return self.remove_pan(g.0 as usize);
            }
            assert!(grab.source_is_touch);
            for i in (g.1 as usize)..(grab.n as usize - 1) {
                grab.coords[i] = grab.coords[i + 1];
            }
        } else {
            return; // shouldn't happen
        }

        // Note: the fact that grab.n > 0 implies source is a touch event!
        for grab in &mut self.touch_grab {
            if grab.pan_grab.0 == g.0 && grab.pan_grab.1 > g.1 {
                grab.pan_grab.1 -= 1;
                if (grab.pan_grab.1 as usize) == MAX_PAN_GRABS - 1 {
                    let v = grab.coord.into();
                    self.pan_grab[g.0 as usize].coords[grab.pan_grab.1 as usize] = (v, v);
                }
            }
        }
    }
}

/// Manager of event-handling and toolkit actions
///
/// A `Manager` is in fact a handle around [`ManagerState`] and [`TkWindow`]
/// in order to provide a convenient user-interface during event processing.
///
/// It exposes two interfaces: one aimed at users implementing widgets and UIs
/// and one aimed at toolkit "frontends". The latter is hidden
/// from documentation unless the `internal_doc` feature is enabled.
#[must_use]
pub struct Manager<'a> {
    read_only: bool,
    mgr: &'a mut ManagerState,
    tkw: &'a mut dyn TkWindow,
    action: TkAction,
}

/// Internal methods
impl<'a> Manager<'a> {
    fn set_hover<W: Widget + ?Sized>(&mut self, widget: &mut W, w_id: Option<WidgetId>) {
        if self.mgr.hover != w_id {
            trace!("Manager: hover = {:?}", w_id);
            self.mgr.hover = w_id;
            self.send_action(TkAction::Redraw);

            if let Some(id) = w_id {
                let icon = widget
                    .find(id)
                    .map(|w| w.cursor_icon())
                    .unwrap_or(CursorIcon::Default);
                if icon != self.mgr.hover_icon {
                    self.mgr.hover_icon = icon;
                    if self.mgr.mouse_grab.is_none() {
                        self.tkw.set_cursor_icon(icon);
                    }
                }
            }
        }
    }

    fn start_key_event<W>(&mut self, widget: &mut W, vkey: VirtualKeyCode, scancode: u32)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        use VirtualKeyCode as VK;
        if self.mgr.char_focus.is_some() {
            match vkey {
                VK::Escape => self.set_char_focus(None),
                _ => (),
            }
            return;
        }

        if vkey == VK::Tab {
            if !self.next_nav_focus(widget.as_widget(), self.mgr.modifiers.shift()) {
                self.clear_nav_focus();
            }
            if let Some(id) = self.mgr.nav_focus {
                self.send_event(widget, id, Event::NavFocus);
            }
        } else if vkey == VK::Escape {
            if let Some(id) = self.mgr.popups.last().map(|(id, _)| *id) {
                self.close_window(id);
            } else {
                self.clear_nav_focus();
            }
        } else {
            let mut id_action = None;

            if !self.mgr.modifiers.alt() {
                // First priority goes to the widget with nav focus,
                // but only when Alt is not pressed.
                if let Some(nav_id) = self.mgr.nav_focus {
                    if vkey == VK::Space || vkey == VK::Return || vkey == VK::NumpadEnter {
                        id_action = Some((nav_id, Event::Activate));
                    } else if let Some(nav_key) = NavKey::new(vkey) {
                        id_action = Some((nav_id, Event::NavKey(nav_key)));
                    }
                }

                if id_action.is_none() {
                    // Next priority goes to pop-up widget
                    if let Some(popup) = self.mgr.popups.last() {
                        if let Some(key) = NavKey::new(vkey) {
                            id_action = Some((popup.1.parent, Event::NavKey(key)));
                        }
                    } else if let Some(id) = self.mgr.nav_fallback {
                        if let Some(key) = NavKey::new(vkey) {
                            id_action = Some((id, Event::NavKey(key)));
                        }
                    }
                }
            }

            if id_action.is_none() {
                // Next priority goes to accelerator keys when Alt is held or alt_bypass is true
                let mut n = 0;
                for (i, id) in (self.mgr.popups.iter().rev())
                    .map(|(_, popup)| popup.parent)
                    .chain(std::iter::once(widget.id()))
                    .enumerate()
                {
                    if let Some(layer) = self.mgr.accel_layers.get(&id) {
                        // but only when Alt is held or alt-bypass is enabled:
                        if self.mgr.modifiers.alt() || layer.0 {
                            if let Some(id) = layer.1.get(&vkey).cloned() {
                                id_action = Some((id, Event::Activate));
                                n = i;
                                break;
                            }
                        }
                    }
                }

                // If we had to look below the top pop-up, we should close it
                if n > 0 {
                    let last = self.mgr.popups.len() - 1;
                    for i in 0..n {
                        let id = self.mgr.popups[last - i].0;
                        self.close_window(id);
                    }
                }
            }

            if let Some((id, event)) = id_action {
                let is_activate = event == Event::Activate;
                self.send_event(widget, id, event);

                // Event::Activate causes buttons to be visually depressed
                if is_activate {
                    for item in &self.mgr.key_depress {
                        if item.1 == id {
                            return;
                        }
                    }

                    self.mgr.key_depress.push((scancode, id));
                    self.redraw(id);
                }
            }
        }
    }

    fn end_key_event(&mut self, scancode: u32) {
        // We must match scancode not vkey since the latter may have changed due to modifiers

        // TODO: it would be nice to replace key_depress with a set
        fn remove<A: smallvec::Array, F: Fn(&A::Item) -> bool>(
            v: &mut SmallVec<A>,
            f: F,
        ) -> Option<A::Item> {
            for (i, item) in v.iter().enumerate() {
                if f(item) {
                    return Some(v.remove(i));
                }
            }
            return None;
        }

        if let Some((_, id)) = remove(&mut self.mgr.key_depress, |item| item.0 == scancode) {
            self.redraw(id);
        }
    }

    fn mouse_grab(&self) -> Option<MouseGrab> {
        self.mgr.mouse_grab.clone()
    }

    fn end_mouse_grab(&mut self, button: MouseButton) {
        if self
            .mgr
            .mouse_grab
            .as_ref()
            .map(|grab| grab.button != button)
            .unwrap_or(true)
        {
            return;
        }
        if let Some(grab) = self.mgr.mouse_grab.take() {
            trace!("Manager: end mouse grab by {}", grab.start_id);
            self.tkw.set_cursor_icon(self.mgr.hover_icon);
            self.redraw(grab.start_id);
            self.mgr.remove_pan_grab(grab.pan_grab);
        }
    }

    #[inline]
    fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchGrab> {
        self.mgr.touch_grab.iter_mut().find_map(|grab| {
            if grab.touch_id == touch_id {
                Some(grab)
            } else {
                None
            }
        })
    }

    fn remove_touch(&mut self, touch_id: u64) -> Option<TouchGrab> {
        let len = self.mgr.touch_grab.len();
        for i in 0..len {
            if self.mgr.touch_grab[i].touch_id == touch_id {
                let grab = self.mgr.touch_grab[i].clone();
                trace!("Manager: end touch grab by {}", grab.start_id);
                self.mgr.touch_grab.swap(i, len - 1);
                self.mgr.touch_grab.truncate(len - 1);
                return Some(grab);
            }
        }
        None
    }

    fn set_char_focus(&mut self, wid: Option<WidgetId>) {
        if self.mgr.char_focus == wid {
            return;
        }

        if let Some(id) = self.mgr.char_focus {
            self.mgr.pending.push(Pending::LostCharFocus(id));
            self.redraw(id);
        }
        if let Some(id) = wid {
            self.redraw(id);
        }
        self.mgr.char_focus = wid;
        trace!("Manager: char_focus = {:?}", wid);
    }

    fn send_event<W: Widget + ?Sized>(&mut self, widget: &mut W, id: WidgetId, event: Event) {
        trace!("Send to {}: {:?}", id, event);
        let _ = widget.send(self, id, event);
    }

    fn send_popup_first<W: Widget + ?Sized>(&mut self, widget: &mut W, id: WidgetId, event: Event) {
        while let Some((wid, parent)) = self.mgr.popups.last().map(|(wid, p)| (*wid, p.parent)) {
            trace!("Send to popup parent: {}: {:?}", parent, event);
            match widget.send(self, parent, event.clone()) {
                Response::Unhandled(_) => (),
                _ => return,
            }
            self.close_window(wid);
        }
        self.send_event(widget, id, event);
    }
}

/// Helper used during widget configuration
pub struct ConfigureManager<'a: 'b, 'b> {
    id: &'b mut WidgetId,
    map: &'b mut HashMap<WidgetId, WidgetId>,
    mgr: &'b mut Manager<'a>,
}

impl<'a: 'b, 'b> ConfigureManager<'a, 'b> {
    /// Reborrow self to pass to a child
    pub fn child<'c>(&'c mut self) -> ConfigureManager<'a, 'c>
    where
        'b: 'c,
    {
        ConfigureManager {
            id: &mut *self.id,
            map: &mut *self.map,
            mgr: &mut *self.mgr,
        }
    }

    /// Get a new [`WidgetId`] for the widget
    ///
    /// Pass the old ID (`self.id()`), even if not yet configured.
    pub fn next_id(&mut self, old_id: WidgetId) -> WidgetId {
        let id = *self.id;
        *self.id = id.next();
        self.map.insert(old_id, id);
        id
    }

    /// Get access to the wrapped [`Manager`]
    pub fn mgr(&mut self) -> &mut Manager<'a> {
        self.mgr
    }
}
