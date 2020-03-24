// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use log::trace;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::u16;

use super::*;
use crate::geom::{Coord, DVec2};
#[allow(unused)]
use crate::WidgetConfig; // for doc-links
use crate::{ThemeAction, ThemeApi, TkAction, TkWindow, Widget, WidgetId, WindowId};

/// Highlighting state of a widget
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct HighlightState {
    /// "Hover" is true if the mouse is over this element or if an active touch
    /// event is over the element.
    pub hover: bool,
    /// Elements such as buttons may be depressed (visually pushed) by a click
    /// or touch event, but in this state the action can still be cancelled.
    /// Elements can also be depressed by keyboard activation.
    ///
    /// If true, this likely implies `hover` is also true.
    pub depress: bool,
    /// Keyboard navigation of UIs moves a "focus" from widget to widget.
    pub nav_focus: bool,
    /// "Character focus" implies this widget is ready to receive text input
    /// (e.g. typing into an input field).
    pub char_focus: bool,
}

impl HighlightState {
    /// True if any part of the state is true
    #[inline]
    pub fn any(self) -> bool {
        self.hover || self.depress || self.nav_focus || self.char_focus
    }
}

/// Controls the types of events delivered by [`Manager::request_grab`]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrabMode {
    /// Deliver [`Event::PressMove`] and [`Event::PressEnd`] for each press
    Grab,
    /// Deliver [`Action::Pan`] events, with scaling and rotation
    PanFull,
    /// Deliver [`Action::Pan`] events, with scaling
    PanScale,
    /// Deliver [`Action::Pan`] events, with rotation
    PanRotate,
    /// Deliver [`Action::Pan`] events, without scaling or rotation
    PanOnly,
}

#[derive(Clone, Debug)]
struct MouseGrab {
    button: MouseButton,
    start_id: WidgetId,
    mode: GrabMode,
    pan_grab: (u16, u16),
}

#[derive(Clone, Debug)]
struct TouchEvent {
    touch_id: u64,
    start_id: WidgetId,
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
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[derive(Clone, Debug)]
pub struct ManagerState {
    dpi_factor: f64,
    char_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_events: SmallVec<[(u32, WidgetId); 10]>,
    last_mouse_coord: Coord,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchEvent; 10]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    accel_keys: HashMap<VirtualKeyCode, WidgetId>,

    time_start: Instant,
    time_updates: Vec<(Instant, WidgetId)>,
    // TODO(opt): consider other containers, e.g. C++ multimap
    // or sorted Vec with binary search yielding a range
    handle_updates: HashMap<UpdateHandle, Vec<WidgetId>>,
    pending: SmallVec<[Pending; 8]>,
    action: TkAction,
}

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl ManagerState {
    /// Construct an event manager per-window data struct
    ///
    /// The DPI factor may be required for event coordinate translation.
    #[inline]
    pub fn new(dpi_factor: f64) -> Self {
        ManagerState {
            dpi_factor,
            char_focus: None,
            nav_focus: None,
            hover: None,
            hover_icon: CursorIcon::Default,
            key_events: Default::default(),
            last_mouse_coord: Coord::ZERO,
            mouse_grab: None,
            touch_grab: Default::default(),
            pan_grab: SmallVec::new(),
            accel_keys: HashMap::new(),

            time_start: Instant::now(),
            time_updates: vec![],
            handle_updates: HashMap::new(),
            pending: SmallVec::new(),
            action: TkAction::None,
        }
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    pub fn configure<W>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        // Re-assigning WidgetIds might invalidate state; to avoid this we map
        // existing ids to new ids
        let mut map = HashMap::new();
        let mut id = WidgetId::FIRST;

        // We re-set these instead of remapping:
        self.accel_keys.clear();
        self.time_updates.clear();
        self.handle_updates.clear();
        self.pending.clear();
        self.action = TkAction::None;

        let coord = self.last_mouse_coord;
        let mut mgr = self.manager(tkw);
        widget.walk_mut(&mut |widget| {
            map.insert(widget.id(), id);
            widget.core_data_mut().id = id;
            widget.configure(&mut mgr);
            id = id.next();
        });

        self.hover = widget.find_id(coord);

        self.char_focus = self.char_focus.and_then(|id| map.get(&id).cloned());
        self.nav_focus = self.nav_focus.and_then(|id| map.get(&id).cloned());
        self.mouse_grab = self.mouse_grab.as_ref().and_then(|grab| {
            map.get(&grab.start_id).map(|id| MouseGrab {
                start_id: *id,
                button: grab.button,
                mode: grab.mode,
                pan_grab: grab.pan_grab,
            })
        });

        let mut i = 0;
        while i < self.pan_grab.len() {
            if let Some(id) = map.get(&self.pan_grab[i].id) {
                self.pan_grab[i].id = *id;
                i += 1;
            } else {
                self.remove_pan(i);
            }
        }
        macro_rules! do_map {
            ($seq:expr, $update:expr) => {
                let update = $update;
                let mut i = 0;
                let mut j = $seq.len();
                while i < j {
                    // invariant: $seq[0..i] have been updated
                    // invariant: $seq[j..len] are rejected
                    if let Some(elt) = update($seq[i].clone()) {
                        $seq[i] = elt;
                        i += 1;
                    } else {
                        j -= 1;
                        $seq.swap(i, j);
                    }
                }
                $seq.truncate(j);
            };
        }

        do_map!(self.touch_grab, |mut elt: TouchEvent| map
            .get(&elt.start_id)
            .map(|id| {
                elt.start_id = *id;
                if let Some(cur_id) = elt.cur_id {
                    elt.cur_id = map.get(&cur_id).cloned();
                }
                elt
            }));

        do_map!(self.key_events, |elt: (u32, WidgetId)| map
            .get(&elt.1)
            .map(|id| (elt.0, *id)));
    }

    /// Update the widgets under the cursor and touch events
    pub fn region_moved<W: Widget + ?Sized>(&mut self, widget: &mut W) {
        // Note: redraw is already implied.

        // Update hovered widget
        self.hover = widget.find_id(self.last_mouse_coord);

        for touch in &mut self.touch_grab {
            touch.cur_id = widget.find_id(touch.coord);
        }
    }

    /// Set the DPI factor. Must be updated for correct event translation by
    /// [`Manager::handle_winit`].
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

    /// Get the next resume time
    pub fn next_resume(&self) -> Option<Instant> {
        self.time_updates.last().map(|time| time.0)
    }

    /// Set an action
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    /// Construct a [`Manager`] referring to this state
    #[inline]
    pub fn manager<'a>(&'a mut self, tkw: &'a mut dyn TkWindow) -> Manager<'a> {
        Manager {
            read_only: false,
            mgr: self,
            tkw,
        }
    }
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

/// Public API (around event manager state)
impl ManagerState {
    /// Get the complete highlight state
    pub fn highlight_state(&self, w_id: WidgetId) -> HighlightState {
        HighlightState {
            hover: self.is_hovered(w_id),
            depress: self.is_depressed(w_id),
            nav_focus: self.nav_focus(w_id),
            char_focus: self.char_focus(w_id),
        }
    }

    /// Get whether this widget has a grab on character input
    #[inline]
    pub fn char_focus(&self, w_id: WidgetId) -> bool {
        self.char_focus == Some(w_id)
    }

    /// Get whether this widget has keyboard focus
    #[inline]
    pub fn nav_focus(&self, w_id: WidgetId) -> bool {
        self.nav_focus == Some(w_id)
    }

    /// Get whether the widget is under the mouse cursor
    #[inline]
    pub fn is_hovered(&self, w_id: WidgetId) -> bool {
        self.mouse_grab.is_none() && self.hover == Some(w_id)
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: WidgetId) -> bool {
        for (_, id) in &self.key_events {
            if *id == w_id {
                return true;
            }
        }
        if let Some(grab) = &self.mouse_grab {
            if grab.start_id == w_id && self.hover == Some(w_id) {
                return true;
            }
        }
        for touch in &self.touch_grab {
            if touch.start_id == w_id && touch.cur_id == Some(w_id) {
                return true;
            }
        }
        false
    }
}

/// Manager of event-handling and toolkit actions
pub struct Manager<'a> {
    read_only: bool,
    mgr: &'a mut ManagerState,
    tkw: &'a mut dyn TkWindow,
}

/// Public API (around toolkit functionality)
impl<'a> Manager<'a> {
    /// Schedule an update
    ///
    /// Widgets requiring animation should schedule an update; as a result,
    /// [`Action::TimerUpdate`] will be sent, roughly at time `now + duration`.
    ///
    /// Timings may be a few ms out, but should be sufficient for e.g. updating
    /// a clock each second. Very short positive durations (e.g. 1ns) may be
    /// used to schedule an update on the next frame. Frames should in any case
    /// be limited by vsync, avoiding excessive frame rates.
    ///
    /// This should be called from [`WidgetConfig::configure`] or from an event
    /// handler. Note that scheduled updates are cleared if reconfigured.
    pub fn update_on_timer(&mut self, duration: Duration, w_id: WidgetId) {
        let time = Instant::now() + duration;
        'outer: loop {
            for row in &mut self.mgr.time_updates {
                if row.1 == w_id {
                    if row.0 <= time {
                        return;
                    } else {
                        row.0 = time;
                        break 'outer;
                    }
                }
            }

            self.mgr.time_updates.push((time, w_id));
            break;
        }

        self.mgr.time_updates.sort_by(|a, b| b.cmp(a)); // reverse sort
    }

    /// Subscribe to an update handle
    ///
    /// All widgets subscribed to an update handle will be sent
    /// [`Action::HandleUpdate`] when [`Manager::trigger_update`]
    /// is called with the corresponding handle.
    ///
    /// This should be called from [`WidgetConfig::configure`].
    pub fn update_on_handle(&mut self, handle: UpdateHandle, w_id: WidgetId) {
        self.mgr
            .handle_updates
            .entry(handle)
            .or_insert(Vec::new())
            .push(w_id);
    }

    /// Notify that a widget must be redrawn
    #[inline]
    pub fn redraw(&mut self, _id: WidgetId) {
        // Theoretically, notifying by WidgetId allows selective redrawing
        // (damage events). This is not yet implemented.
        self.send_action(TkAction::Redraw);
    }

    /// Notify that a [`TkAction`] action should happen
    ///
    /// This causes the given action to happen after event handling.
    ///
    /// Whenever a widget is added, removed or replaced, a reconfigure action is
    /// required. Should a widget's size requirements change, these will only
    /// affect the UI after a reconfigure action.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.mgr.send_action(action);
    }

    /// Add a pop-up
    ///
    /// A pop-up is a box used for things like tool-tips and menus which is
    /// drawn on top of other content and has focus for input.
    ///
    /// Depending on the host environment, the pop-up may be a special type of
    /// window without borders and with precise placement, or may be a layer
    /// drawn in an existing window.
    #[inline]
    // TODO: parameters
    pub fn add_overlay(&mut self, widget: Box<dyn kas::Overlay>) {
        self.tkw.add_overlay(widget);
    }

    /// Add a window
    ///
    /// Toolkits typically allow windows to be added directly, before start of
    /// the event loop (e.g. `kas_wgpu::Toolkit::add`).
    ///
    /// This method is an alternative allowing a window to be added via event
    /// processing, albeit without error handling.
    #[inline]
    pub fn add_window(&mut self, widget: Box<dyn kas::Window>) -> WindowId {
        self.tkw.add_window(widget)
    }

    /// Close a window
    #[inline]
    pub fn close_window(&mut self, id: WindowId) {
        self.tkw.close_window(id);
    }

    /// Updates all subscribed widgets
    ///
    /// All widgets subscribed to the given [`UpdateHandle`], across all
    /// windows, will receive an update.
    #[inline]
    pub fn trigger_update(&mut self, handle: UpdateHandle, payload: u64) {
        self.tkw.trigger_update(handle, payload);
    }

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    #[inline]
    pub fn get_clipboard(&mut self) -> Option<crate::CowString> {
        self.tkw.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard<'c>(&mut self, content: crate::CowStringL<'c>) {
        self.tkw.set_clipboard(content)
    }

    /// Adjust the theme
    #[inline]
    pub fn adjust_theme<F: FnMut(&mut dyn ThemeApi) -> ThemeAction>(&mut self, mut f: F) {
        self.tkw.adjust_theme(&mut f);
    }
}

/// Public API (around event manager state)
impl<'a> Manager<'a> {
    /// Adds an accelerator key for a widget
    ///
    /// If this key is pressed when the window has focus and no widget has a
    /// key-grab, the given widget will receive an [`Action::Activate`] event.
    ///
    /// This should be set from [`WidgetConfig::configure`].
    #[inline]
    pub fn add_accel_key(&mut self, key: VirtualKeyCode, id: WidgetId) {
        if !self.read_only {
            self.mgr.accel_keys.insert(key, id);
        }
    }

    /// Request character-input focus
    ///
    /// If successful, [`Action::ReceivedCharacter`] events are sent to this
    /// widget when character data is received.
    ///
    /// Currently, this method always succeeds.
    pub fn request_char_focus(&mut self, id: WidgetId) {
        if !self.read_only {
            self.set_char_focus(Some(id));
        }
    }

    /// Request a grab on the given input `source`
    ///
    /// If successful, corresponding mouse/touch events will be forwarded to
    /// this widget. The grab terminates automatically.
    ///
    /// Behaviour depends on the `mode`:
    ///
    /// -   [`GrabMode::Grab`]: simple / low-level interpretation of input
    ///     which delivers [`Event::PressMove`] and [`Event::PressEnd`] events.
    ///     Multiple event sources may be grabbed simultaneously.
    /// -   All other [`GrabMode`] values: generates [`Action::Pan`] events.
    ///     Requesting additional grabs on the same widget from the same source
    ///     (i.e. multiple touches) allows generation of rotation and scale
    ///     factors (depending on the [`GrabMode`]).
    ///     Any previously existing `Pan` grabs by this widgets are replaced.
    ///
    /// Since these events are *requested*, the widget should consume them even
    /// if not required (e.g. [`Event::PressMove`], although in practice this
    /// only affects parents intercepting [`Response::Unhandled`] events.
    ///
    /// This method normally succeeds, but fails when
    /// multiple widgets attempt a grab the same press source simultaneously
    /// (only the first grab is successful).
    ///
    /// This method automatically cancels any active char grab
    /// and updates keyboard navigation focus.
    pub fn request_grab(
        &mut self,
        id: WidgetId,
        source: PressSource,
        coord: Coord,
        mode: GrabMode,
        cursor: Option<CursorIcon>,
    ) -> bool {
        if self.read_only {
            return false;
        }

        let start_id = id;
        let mut pan_grab = (u16::MAX, 0);
        match source {
            PressSource::Mouse(button) => {
                if self.mgr.mouse_grab.is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.mgr.set_pan_on(id, mode, false, coord);
                }
                self.mgr.mouse_grab = Some(MouseGrab {
                    start_id,
                    button,
                    mode,
                    pan_grab,
                });
                if let Some(icon) = cursor {
                    self.tkw.set_cursor_icon(icon);
                }
            }
            PressSource::Touch(touch_id) => {
                if self.get_touch(touch_id).is_some() {
                    return false;
                }
                if mode != GrabMode::Grab {
                    pan_grab = self.mgr.set_pan_on(id, mode, true, coord);
                }
                self.mgr.touch_grab.push(TouchEvent {
                    touch_id,
                    start_id,
                    cur_id: Some(id),
                    coord,
                    mode,
                    pan_grab,
                });
            }
        }

        self.set_char_focus(None);
        self.redraw(start_id);
        true
    }
}

/// Internal methods
impl<'a> Manager<'a> {
    #[cfg(feature = "winit")]
    fn set_hover<W: Widget + ?Sized>(&mut self, widget: &mut W, w_id: Option<WidgetId>) {
        if self.mgr.hover != w_id {
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
    fn add_key_event(&mut self, scancode: u32, id: WidgetId) {
        for item in &self.mgr.key_events {
            if item.1 == id {
                return;
            }
        }

        self.mgr.key_events.push((scancode, id));
        self.redraw(id);
    }

    #[cfg(feature = "winit")]
    fn remove_key_event(&mut self, scancode: u32) {
        let r = 'outer: loop {
            for (i, item) in self.mgr.key_events.iter().enumerate() {
                // We must match scancode not vkey since the
                // latter may have changed due to modifiers
                if item.0 == scancode {
                    break 'outer i;
                }
            }
            return;
        };
        self.redraw(self.mgr.key_events[r].1);
        self.mgr.key_events.remove(r);
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
            self.tkw.set_cursor_icon(self.mgr.hover_icon);
            self.redraw(grab.start_id);
            self.mgr.remove_pan_grab(grab.pan_grab);
        }
    }

    #[inline]
    fn get_touch(&mut self, touch_id: u64) -> Option<&mut TouchEvent> {
        self.mgr.touch_grab.iter_mut().find_map(|grab| {
            if grab.touch_id == touch_id {
                Some(grab)
            } else {
                None
            }
        })
    }

    #[cfg(feature = "winit")]
    fn remove_touch(&mut self, touch_id: u64) -> Option<TouchEvent> {
        let len = self.mgr.touch_grab.len();
        for i in 0..len {
            if self.mgr.touch_grab[i].touch_id == touch_id {
                let grab = self.mgr.touch_grab[i].clone();
                self.mgr.touch_grab.swap(i, len - 1);
                self.mgr.touch_grab.truncate(len - 1);
                return Some(grab);
            }
        }
        None
    }

    #[cfg(feature = "winit")]
    fn next_nav_focus<W: Widget + ?Sized>(&mut self, widget: &mut W) {
        let mut id = self.mgr.nav_focus.unwrap_or(WidgetId::FIRST);
        let end = widget.id();
        loop {
            id = id.next();
            if id >= end {
                return self.unset_nav_focus();
            }

            // TODO(opt): incorporate walk/find logic
            if widget.find(id).map(|w| w.key_nav()).unwrap_or(false) {
                self.send_action(TkAction::Redraw);
                self.mgr.nav_focus = Some(id);
                return;
            }
        }
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
    }
}

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<'a> Manager<'a> {
    // Hack to make TkAction::Close on an overlay close only that.
    // This is not quite correct, since it could mask a legitimate Close
    // event (e.g. a pop-up menu to close the window).
    pub(crate) fn replace_action_close_with_reconfigure(&mut self) -> bool {
        if self.mgr.action == TkAction::Close {
            self.mgr.action = TkAction::Reconfigure;
            true
        } else {
            false
        }
    }

    /// Update, after receiving all events
    pub fn finish<W>(mut self, widget: &mut W) -> TkAction
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        for gi in 0..self.mgr.pan_grab.len() {
            let grab = &mut self.mgr.pan_grab[gi];
            debug_assert!(grab.mode != GrabMode::Grab);
            assert!(grab.n > 0);

            // Terminology: pi are old coordinates, qi are new coords
            let (p1, q1) = (DVec2::from(grab.coords[0].0), DVec2::from(grab.coords[0].1));
            grab.coords[0].0 = grab.coords[0].1;

            let alpha;
            let delta;

            if grab.mode == GrabMode::PanOnly || grab.n == 1 {
                alpha = DVec2(1.0, 0.0);
                delta = DVec2::from(q1 - p1);
            } else {
                // We don't use more than two touches: information would be
                // redundant (although it could be averaged).
                let (p2, q2) = (DVec2::from(grab.coords[1].0), DVec2::from(grab.coords[1].1));
                grab.coords[1].0 = grab.coords[1].1;
                let (pd, qd) = (p2 - p1, q2 - q1);

                alpha = match grab.mode {
                    GrabMode::PanFull => qd.complex_div(pd),
                    GrabMode::PanScale => DVec2((qd.sum_square() / pd.sum_square()).sqrt(), 0.0),
                    GrabMode::PanRotate => {
                        let a = qd.complex_div(pd);
                        a / a.sum_square().sqrt()
                    }
                    _ => unreachable!(),
                };

                // Average delta from both movements:
                delta = (q1 - alpha.complex_mul(p1) + q2 - alpha.complex_mul(p2)) * 0.5;
            }

            let id = grab.id;
            if alpha != DVec2(1.0, 0.0) || delta != DVec2::ZERO {
                let ev = Event::Action(Action::Pan { alpha, delta });
                let _ = widget.event(&mut self, id, ev);
            }
        }

        // To avoid infinite loops, we consider self read-only from here on.
        // Since we don't wish to duplicate Handler::handle, we don't actually
        // make self const, but merely pretend it is in the public API.
        self.read_only = true;

        for item in self.mgr.pending.pop() {
            match item {
                Pending::LostCharFocus(id) => {
                    let ev = Event::Action(Action::LostCharFocus);
                    let _ = widget.event(&mut self, id, ev);
                }
            }
        }

        let action = self.mgr.action;
        self.mgr.action = TkAction::None;
        action
    }

    /// Update widgets due to timer
    pub fn update_timer<W: Widget + ?Sized>(&mut self, widget: &mut W) {
        let now = Instant::now();

        // assumption: time_updates are sorted in reverse order
        while !self.mgr.time_updates.is_empty() {
            if self.mgr.time_updates.last().unwrap().0 > now {
                break;
            }

            let update = self.mgr.time_updates.pop().unwrap();
            let w_id = update.1;
            let action = Action::TimerUpdate;
            trace!("Sending {:?} to widget {}", action, w_id);
            let _ = widget.event(self, w_id, Event::Action(action));
        }

        self.mgr.time_updates.sort_by(|a, b| b.cmp(a)); // reverse sort
    }

    /// Update widgets due to handle
    pub fn update_handle<W: Widget + ?Sized>(
        &mut self,
        widget: &mut W,
        handle: UpdateHandle,
        payload: u64,
    ) {
        // NOTE: to avoid borrow conflict, we must clone values!
        if let Some(mut values) = self.mgr.handle_updates.get(&handle).cloned() {
            for w_id in values.drain(..) {
                let action = Action::HandleUpdate { handle, payload };
                trace!("Sending {:?} to widget {}", action, w_id);
                let _ = widget.event(self, w_id, Event::Action(action));
            }
        }
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(&mut self, widget: &mut W, event: winit::event::WindowEvent)
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};
        trace!("Event: {:?}", event);

        // Note: since <W as Handler>::Msg = VoidMsg, only two values of
        // Response are possible: None and Unhandled. We don't have any use for
        // Unhandled events here, so we can freely ignore all responses.

        match event {
            // Resized(size) [handled by toolkit]
            // Moved(position)
            CloseRequested => {
                self.send_action(TkAction::Close);
            }
            // Destroyed
            // DroppedFile(PathBuf),
            // HoveredFile(PathBuf),
            // HoveredFileCancelled,
            ReceivedCharacter(c) if c != '\u{1b}' /* escape */ => {
                if let Some(id) = self.mgr.char_focus {
                    let ev = Event::Action(Action::ReceivedCharacter(c));
                    let _ = widget.event(self, id, ev);
                }
            }
            // Focused(bool),
            KeyboardInput { input, is_synthetic, .. } => {
                let char_focus = self.mgr.char_focus.is_some();
                if input.state == ElementState::Pressed && !is_synthetic {
                    if let Some(vkey) = input.virtual_keycode {
                        if char_focus {
                            match vkey {
                                VirtualKeyCode::Escape => self.set_char_focus(None),
                                _ => (),
                            }
                        } else {
                            match (vkey, self.mgr.nav_focus) {
                                (VirtualKeyCode::Tab, _) => {
                                    self.next_nav_focus(widget);
                                }
                                (VirtualKeyCode::Space, Some(nav_id)) |
                                (VirtualKeyCode::Return, Some(nav_id)) |
                                (VirtualKeyCode::NumpadEnter, Some(nav_id))  => {
                                    // Add to key_events for visual feedback
                                    self.add_key_event(input.scancode, nav_id);

                                    let ev = Event::Action(Action::Activate);
                                    let _ = widget.event(self, nav_id, ev);
                                }
                                (VirtualKeyCode::Escape, _) => self.unset_nav_focus(),
                                (vkey, _) => {
                                    if let Some(id) = self.mgr.accel_keys.get(&vkey).cloned() {
                                        // Add to key_events for visual feedback
                                        self.add_key_event(input.scancode, id);

                                        let ev = Event::Action(Action::Activate);
                                        let _ = widget.event(self, id, ev);
                                    }
                                }
                            }
                        }
                    }
                } else if input.state == ElementState::Released {
                    self.remove_key_event(input.scancode);
                }
            }
            CursorMoved {
                position,
                ..
            } => {
                let coord = position.into();

                // Update hovered widget
                self.set_hover(widget, widget.find_id(coord));

                if let Some(grab) = self.mouse_grab() {
                    let delta = coord - self.mgr.last_mouse_coord;
                    if grab.mode == GrabMode::Grab {
                        let source = PressSource::Mouse(grab.button);
                        let ev = Event::PressMove { source, coord, delta };
                        let _ = widget.event(self, grab.start_id, ev);
                    } else {
                        if let Some(pan) = self.mgr.pan_grab.get_mut(grab.pan_grab.0 as usize) {
                            pan.coords[grab.pan_grab.1 as usize].1 = coord;
                        }
                    }
                } else {
                    // We don't forward move events without a grab
                }

                self.mgr.last_mouse_coord = coord;
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                if self.mouse_grab().is_none() {
                    // If there's a mouse grab, we will continue to receive
                    // coordinates; if not, set a fake coordinate off the window
                    self.mgr.last_mouse_coord = Coord(-1, -1);
                    self.set_hover(widget, None);
                }
            }
            MouseWheel { delta, .. } => {
                let action = Action::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) =>
                        ScrollDelta::PixelDelta(Coord::from_logical(pos, self.mgr.dpi_factor)),
                });
                if let Some(id) = self.mgr.hover {
                    let _ = widget.event(self, id, Event::Action(action));
                }
            }
            MouseInput {
                state,
                button,
                ..
            } => {
                let coord = self.mgr.last_mouse_coord;
                let source = PressSource::Mouse(button);

                if let Some(grab) = self.mouse_grab() {
                    match grab.mode {
                        GrabMode::Grab => {
                            // Mouse grab active: send events there
                            let ev = match state {
                                ElementState::Pressed => Event::PressStart { source, coord },
                                ElementState::Released => Event::PressEnd {
                                    source,
                                    end_id: self.mgr.hover,
                                    coord,
                                },
                            };
                            let _ = widget.event(self, grab.start_id, ev);
                        }
                        // Pan events do not receive Start/End notifications
                        _ => (),
                    };

                    if state == ElementState::Released {
                        self.end_mouse_grab(button);
                    }
                } else if let Some(id) = self.mgr.hover {
                    // No mouse grab but have a hover target
                    if state == ElementState::Pressed {
                        let ev = Event::PressStart { source, coord };
                        let _ = widget.event(self, id, ev);
                    }
                }
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            // RedrawRequested [handled by toolkit]
            Touch(touch) => {
                let source = PressSource::Touch(touch.id);
                let coord = touch.location.into();
                match touch.phase {
                    TouchPhase::Started => {
                        if let Some(id) = widget.find_id(coord) {
                            let ev = Event::PressStart { source, coord };
                            let _ = widget.event(self, id, ev);
                        }
                    }
                    TouchPhase::Moved => {
                        // NOTE: calling widget.event twice appears
                        // to be unavoidable (as with CursorMoved)
                        let cur_id = widget.find_id(coord);

                        let mut r = None;
                        let mut pan_grab = None;
                        if let Some(grab) = self.get_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                let id = grab.start_id;
                                let action = Event::PressMove {
                                    source,
                                    coord,
                                    delta: coord - grab.coord,
                                };
                                // Only when 'depressed' status changes:
                                let redraw = grab.cur_id != cur_id &&
                                    (grab.cur_id == Some(grab.start_id) || cur_id == Some(grab.start_id));

                                grab.cur_id = cur_id;
                                grab.coord = coord;

                                r = Some((id, action, redraw));
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if let Some((id, action, redraw)) = r {
                            if redraw {
                                self.send_action(TkAction::Redraw);
                            }
                            let _ = widget.event(self, id, action);
                        } else if let Some(pan_grab) = pan_grab {
                            if (pan_grab.1 as usize) < MAX_PAN_GRABS {
                                if let Some(pan) = self.mgr.pan_grab.get_mut(pan_grab.0 as usize) {
                                    pan.coords[pan_grab.1 as usize].1 = coord;
                                }
                            }
                        }
                    }
                    TouchPhase::Ended => {
                        if let Some(grab) = self.remove_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                let action = Event::PressEnd {
                                    source,
                                    end_id: grab.cur_id,
                                    coord,
                                };
                                if let Some(cur_id) = grab.cur_id {
                                    self.redraw(cur_id);
                                }
                                let _ = widget.event(self, grab.start_id, action);
                            } else {
                                self.mgr.remove_pan_grab(grab.pan_grab);
                            }
                        }
                    }
                    TouchPhase::Cancelled => {
                        if let Some(grab) = self.remove_touch(touch.id) {
                            let action = Event::PressEnd {
                                source,
                                end_id: None,
                                coord,
                            };
                            if let Some(cur_id) = grab.cur_id {
                                self.redraw(cur_id);
                            }
                            let _ = widget.event(self, grab.start_id, action);
                        }
                    }
                }
            }
            // HiDpiFactorChanged(factor) [handled by toolkit]
            _ => (),
        }
    }
}
