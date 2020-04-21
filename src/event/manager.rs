// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

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

/// Window event manager
///
/// Encapsulation of per-window event state plus supporting methods.
///
/// This structure additionally tracks animated widgets (those requiring
/// periodic update).
//
// Note that the most frequent usage of fields is to check highlighting states
// drawing redraw, which requires iterating all grab & key events.
// Thus for these collections, the preferred container is SmallVec.
#[derive(Debug)]
pub struct ManagerState {
    dpi_factor: f64,
    modifiers: ModifiersState,
    char_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    nav_stack: SmallVec<[u32; 16]>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_depress: SmallVec<[(u32, WidgetId); 10]>,
    last_mouse_coord: Coord,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 10]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    accel_keys: HashMap<VirtualKeyCode, WidgetId>,
    popups: SmallVec<[(WindowId, kas::Popup); 16]>,

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

    #[cfg(feature = "winit")]
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

    #[cfg(feature = "winit")]
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
            self.next_nav_focus(widget.as_widget(), self.mgr.modifiers.shift());
        } else if vkey == VK::Escape {
            self.unset_nav_focus();
        } else {
            let mut id_action = None;

            if let Some(nav_id) = self.mgr.nav_focus {
                if vkey == VK::Space || vkey == VK::Return || vkey == VK::NumpadEnter {
                    id_action = Some((nav_id, Event::Activate));
                } else if let Some(nav_key) = NavKey::new(vkey) {
                    id_action = Some((nav_id, Event::NavKey(nav_key)));
                }
            }

            if id_action.is_none() {
                if let Some(id) = self.mgr.accel_keys.get(&vkey).cloned() {
                    id_action = Some((id, Event::Activate));
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

    #[cfg(feature = "winit")]
    fn end_key_event(&mut self, scancode: u32) {
        let r = 'outer: loop {
            for (i, item) in self.mgr.key_depress.iter().enumerate() {
                // We must match scancode not vkey since the
                // latter may have changed due to modifiers
                if item.0 == scancode {
                    break 'outer i;
                }
            }
            return;
        };
        self.redraw(self.mgr.key_depress[r].1);
        self.mgr.key_depress.remove(r);
    }

    #[cfg(feature = "winit")]
    fn mouse_grab(&self) -> Option<MouseGrab> {
        self.mgr.mouse_grab.clone()
    }

    #[cfg(feature = "winit")]
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

    #[cfg(feature = "winit")]
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

    #[cfg(feature = "winit")]
    fn next_nav_focus<'b>(&mut self, mut widget: &'b dyn WidgetConfig, backward: bool) {
        type WidgetStack<'b> = SmallVec<[&'b dyn WidgetConfig; 16]>;
        let mut widget_stack = WidgetStack::new();

        // Reconstruct widget_stack:
        for index in self.mgr.nav_stack.iter().cloned() {
            let new = widget.get(index as usize).unwrap();
            widget_stack.push(widget);
            widget = new;
        }

        // Progresses to the first child (or last if backward).
        // Returns true if a child is found.
        // Breaks to given lifetime on error.
        macro_rules! do_child {
            ($lt:lifetime, $nav_stack:ident, $widget:ident, $widget_stack:ident) => {{
                let range = $widget.spatial_range();
                if $widget.is_disabled() || range.1 == std::usize::MAX {
                    false
                } else {
                    // We have a child; the first is range.0 unless backward
                    let index = match backward {
                        false => range.0,
                        true => range.1,
                    };
                    let new = match $widget.get(index) {
                        None => break $lt,
                        Some(w) => w,
                    };
                    $nav_stack.push(index as u32);
                    $widget_stack.push($widget);
                    $widget = new;
                    true
                }
            }};
        };

        // Progresses to the next (or previous) sibling, otherwise pops to the
        // parent. Returns true if a sibling is found.
        // Breaks to given lifetime on error.
        macro_rules! do_sibling_or_pop {
            ($lt:lifetime, $nav_stack:ident, $widget:ident, $widget_stack:ident) => {{
                let mut index;
                match ($nav_stack.pop(), $widget_stack.pop()) {
                    (Some(i), Some(w)) => {
                        index = i as usize;
                        $widget = w;
                    }
                    _ => break $lt,
                };
                let mut range = $widget.spatial_range();
                if $widget.is_disabled() || range.1 == std::usize::MAX {
                    break $lt;
                }

                let backward = (range.1 < range.0) ^ backward;
                if range.1 < range.0 {
                    std::mem::swap(&mut range.0, &mut range.1);
                }

                // Look for next sibling
                let have_sibling = match backward {
                    false if index < range.1 => {
                        index += 1;
                        true
                    }
                    true if range.0 < index => {
                        index -= 1;
                        true
                    }
                    _ => false,
                };

                if have_sibling {
                    let new = match $widget.get(index) {
                        None => break $lt,
                        Some(w) => w,
                    };
                    $nav_stack.push(index as u32);
                    $widget_stack.push($widget);
                    $widget = new;
                }
                have_sibling
            }};
        };

        // We redraw in all cases. Since this is not part of widget event
        // processing, we can push directly to self.mgr.action.
        self.mgr.send_action(TkAction::Redraw);
        let nav_stack = &mut self.mgr.nav_stack;

        if !backward {
            // Depth-first search without function recursion. Our starting
            // entry has already been used (if applicable); the next
            // candidate is its first child.
            'l1: loop {
                if do_child!('l1, nav_stack, widget, widget_stack) {
                    if widget.key_nav() && !widget.is_disabled() {
                        self.mgr.nav_focus = Some(widget.id());
                        trace!("Manager: nav_focus = {:?}", self.mgr.nav_focus);
                        return;
                    }
                    continue;
                }

                loop {
                    if do_sibling_or_pop!('l1, nav_stack, widget, widget_stack) {
                        if widget.key_nav() && !widget.is_disabled() {
                            self.mgr.nav_focus = Some(widget.id());
                            trace!("Manager: nav_focus = {:?}", self.mgr.nav_focus);
                            return;
                        }
                        break;
                    }
                }
            }
        } else {
            // Reverse depth-first search
            let mut start = self.mgr.nav_focus.is_none();
            'l2: loop {
                if start || do_sibling_or_pop!('l2, nav_stack, widget, widget_stack) {
                    start = false;
                    while do_child!('l2, nav_stack, widget, widget_stack) {}
                }

                if widget.key_nav() && !widget.is_disabled() {
                    self.mgr.nav_focus = Some(widget.id());
                    trace!("Manager: nav_focus = {:?}", self.mgr.nav_focus);
                    return;
                }
            }
        }

        // We end up here when there are no more nodes to walk and when
        // an error occurs (unable to get widget within spatial_range).
        self.mgr.nav_stack.clear();
        self.mgr.nav_focus = None;
        trace!("Manager: nav_focus = {:?}", self.mgr.nav_focus);
    }

    #[cfg(feature = "winit")]
    fn unset_nav_focus(&mut self) {
        if let Some(id) = self.mgr.nav_focus {
            self.redraw(id);
        }
        self.mgr.nav_focus = None;
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

    #[cfg(feature = "winit")]
    fn send_press_start<W: Widget + ?Sized>(
        &mut self,
        widget: &mut W,
        source: PressSource,
        start_id: WidgetId,
        coord: Coord,
    ) {
        let event = Event::PressStart {
            source,
            start_id,
            coord,
        };
        if let Some(id) = self.mgr.popups.last().map(|(_, p)| p.parent) {
            trace!("Send to popup parent: {}: {:?}", id, event);
            match widget.send(self, id, event.clone()) {
                Response::Unhandled(_) => (),
                _ => return,
            }
        }
        self.send_event(widget, start_id, event);
    }
}
