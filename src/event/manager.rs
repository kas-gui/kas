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
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[derive(Clone, Debug)]
pub struct ManagerState {
    dpi_factor: f64,
    modifiers: ModifiersState,
    char_focus: Option<WidgetId>,
    nav_focus: Option<WidgetId>,
    nav_stack: SmallVec<[u32; 16]>,
    hover: Option<WidgetId>,
    hover_icon: CursorIcon,
    key_events: SmallVec<[(u32, WidgetId); 10]>,
    last_mouse_coord: Coord,
    mouse_grab: Option<MouseGrab>,
    touch_grab: SmallVec<[TouchGrab; 10]>,
    pan_grab: SmallVec<[PanGrab; 4]>,
    press_focus: Option<WidgetId>,
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
            modifiers: ModifiersState::empty(),
            char_focus: None,
            nav_focus: None,
            nav_stack: SmallVec::new(),
            hover: None,
            hover_icon: CursorIcon::Default,
            key_events: Default::default(),
            last_mouse_coord: Coord::ZERO,
            mouse_grab: None,
            touch_grab: Default::default(),
            pan_grab: SmallVec::new(),
            press_focus: None,
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
        trace!("Manager::configure");

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
        self.with(tkw, |mut mgr| {
            widget.walk_mut(&mut |widget| {
                map.insert(widget.id(), id);
                widget.core_data_mut().id = id;
                widget.configure(&mut mgr);
                id = id.next();
            });
            let hover = widget.find_id(coord);
            mgr.set_hover(widget, hover);
        });

        self.char_focus = self.char_focus.and_then(|id| map.get(&id).cloned());
        self.nav_focus = self.nav_focus.and_then(|id| map.get(&id).cloned());
        self.press_focus = self.press_focus.and_then(|id| map.get(&id).cloned());
        self.mouse_grab = self.mouse_grab.as_ref().and_then(|grab| {
            map.get(&grab.start_id).map(|id| MouseGrab {
                start_id: *id,
                depress: grab.depress.and_then(|id| map.get(&id).cloned()),
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

        do_map!(self.touch_grab, |mut elt: TouchGrab| map
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
    pub fn region_moved<W: Widget + ?Sized>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W) {
        trace!("Manager::region_moved");
        // Note: redraw is already implied.

        self.nav_focus = self
            .nav_focus
            .and_then(|id| widget.find(id).map(|w| w.id()));
        self.char_focus = self
            .char_focus
            .and_then(|id| widget.find(id).map(|w| w.id()));

        // Update hovered widget
        let hover = widget.find_id(self.last_mouse_coord);
        self.with(tkw, |mgr| mgr.set_hover(widget, hover));

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
    ///
    /// Since this is a commonly used operation, an operator overload is
    /// available to do this job: `mgr << action;` or even `mgr << a << b;`.
    #[inline]
    pub fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }

    /// Construct a [`Manager`] referring to this state
    ///
    /// Invokes the given closure on this [`Manager`].
    #[inline]
    pub fn with<F>(&mut self, tkw: &mut dyn TkWindow, f: F)
    where
        F: FnOnce(&mut Manager),
    {
        let mut mgr = Manager {
            read_only: false,
            mgr: self,
            tkw,
            action: TkAction::None,
        };
        f(&mut mgr);
        let action = mgr.action;
        self.send_action(action);
    }

    /// Update, after receiving all events
    #[inline]
    pub fn update<W>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W) -> TkAction
    where
        W: Widget<Msg = VoidMsg> + ?Sized,
    {
        let mgr = Manager {
            read_only: false,
            mgr: self,
            tkw,
            action: TkAction::None,
        };
        // we could inline this:
        mgr.update(widget)
    }
}

impl<'a> std::ops::AddAssign<TkAction> for Manager<'a> {
    #[inline]
    fn add_assign(&mut self, action: TkAction) {
        self.send_action(action);
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

/// Public API (around event manager state)
impl ManagerState {
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
        if self.mouse_grab.as_ref().and_then(|grab| grab.depress) == Some(w_id) {
            return true;
        }
        for touch in &self.touch_grab {
            if touch.depress == Some(w_id) {
                return true;
            }
        }
        false
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

/// Public API (around toolkit functionality)
impl<'a> Manager<'a> {
    /// Schedule an update
    ///
    /// Widgets requiring animation should schedule an update; as a result,
    /// [`Event::TimerUpdate`] will be sent, roughly at time `now + duration`.
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
    /// [`Event::HandleUpdate`] when [`Manager::trigger_update`]
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
        self.action = self.action.max(action);
    }

    /// Get the current [`TkAction`], replacing with `None`
    ///
    /// The caller is responsible for ensuring the action is handled correctly;
    /// generally this means matching only actions which can be handled locally
    /// and downgrading the action, adding the result back to the [`Manager`].
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    /// Add an overlay (pop-up)
    ///
    /// A pop-up is a box used for things like tool-tips and menus which is
    /// drawn on top of other content and has focus for input.
    ///
    /// Depending on the host environment, the pop-up may be a special type of
    /// window without borders and with precise placement, or may be a layer
    /// drawn in an existing window.
    ///
    /// The pop-up should be placed *next to* the specified `rect`, in the given
    /// `direction`.
    #[inline]
    pub fn add_popup(&mut self, popup: kas::Popup) -> WindowId {
        self.tkw.add_popup(popup)
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
    /// key-grab, the given widget will receive an [`Event::Activate`] event.
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
    /// If successful, [`Event::ReceivedCharacter`] events are sent to this
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
    /// Each grab can optionally visually depress one widget, and initially
    /// depresses the widget owning the grab (the `id` passed here). Call
    /// [`Manager::set_grab_depress`] to update the grab's depress target.
    /// This is cleared automatically when the grab ends.
    ///
    /// Behaviour depends on the `mode`:
    ///
    /// -   [`GrabMode::Grab`]: simple / low-level interpretation of input
    ///     which delivers [`Event::PressMove`] and [`Event::PressEnd`] events.
    ///     Multiple event sources may be grabbed simultaneously.
    /// -   All other [`GrabMode`] values: generates [`Event::Pan`] events.
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
                trace!("Manager: start mouse grab by {}", start_id);
                self.mgr.mouse_grab = Some(MouseGrab {
                    start_id,
                    depress: Some(id),
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
                trace!("Manager: start touch grab by {}", start_id);
                self.mgr.touch_grab.push(TouchGrab {
                    touch_id,
                    start_id,
                    depress: Some(id),
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

    /// Set a grab's depress target
    ///
    /// When a grab on mouse or touch input is in effect
    /// ([`Manager::request_grab`]), the widget owning the grab may set itself
    /// or any other widget as *depressed* ("pushed down"). Each grab depresses
    /// at most one widget, thus setting a new depress target clears any
    /// existing target. Initially a grab depresses its owner.
    ///
    /// This effect is purely visual. A widget is depressed when one or more
    /// grabs targets the widget to depress, or when a keyboard binding is used
    /// to activate a widget (for the duration of the key-press).
    pub fn set_grab_depress(&mut self, source: PressSource, target: Option<WidgetId>) {
        match source {
            PressSource::Mouse(_) => {
                if let Some(grab) = self.mgr.mouse_grab.as_mut() {
                    grab.depress = target;
                }
            }
            PressSource::Touch(id) => {
                for touch in &mut self.mgr.touch_grab {
                    if touch.touch_id == id {
                        touch.depress = target;
                        break;
                    }
                }
            }
        }
    }

    /// Request press focus priority
    ///
    /// The target widget will receive all new [`Event::PressStart`] events
    /// until either press focus is explicitly cleared or the handler returns
    /// [`Response::Unhandled`]. In the latter case, the actual payload of
    /// `Unhandled` is ignored and the original event is sent to the usual
    /// recipient (without press focus).
    ///
    /// Additionally, the target widget will receive [`Event::PressMove`]
    /// events when the mouse cursor moves, even without a grab. (This does not
    /// apply to touch events since these cannot occur without `PressStart`.
    /// Also note that the `source` component uses a fake button!)
    pub fn set_press_focus(&mut self, target: Option<WidgetId>) {
        self.mgr.press_focus = target;
    }
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
                self.send_event(widget, id, event);

                // Add to key_events for visual feedback
                for item in &self.mgr.key_events {
                    if item.1 == id {
                        return;
                    }
                }

                self.mgr.key_events.push((scancode, id));
                self.redraw(id);
            }
        }
    }

    #[cfg(feature = "winit")]
    fn end_key_event(&mut self, scancode: u32) {
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
}

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<'a> Manager<'a> {
    fn update<W>(mut self, widget: &mut W) -> TkAction
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
                let event = Event::Pan { alpha, delta };
                self.send_event(widget, id, event);
            }
        }

        // To avoid infinite loops, we consider self read-only from here on.
        // Since we don't wish to duplicate Handler::handle, we don't actually
        // make self const, but merely pretend it is in the public API.
        self.read_only = true;

        for item in self.mgr.pending.pop() {
            match item {
                Pending::LostCharFocus(id) => {
                    let event = Event::LostCharFocus;
                    self.send_event(widget, id, event);
                }
            }
        }

        let action = self.mgr.action + self.action;
        self.mgr.action = TkAction::None;
        action
    }

    fn send_event<W: Widget + ?Sized>(&mut self, widget: &mut W, id: WidgetId, event: Event) {
        trace!("Send to {}: {:?}", id, event);
        let _ = widget.send(self, id, event);
    }

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
        if let Some(id) = self.mgr.press_focus {
            trace!("Send to press focus target: {}: {:?}", id, event);
            match widget.send(self, id, event.clone()) {
                Response::Unhandled(_) => (),
                _ => return,
            }
        }
        self.send_event(widget, start_id, event);
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
            self.send_event(widget, update.1, Event::TimerUpdate);
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
                let event = Event::HandleUpdate { handle, payload };
                self.send_event(widget, w_id, event);
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
                    let event = Event::ReceivedCharacter(c);
                    self.send_event(widget, id, event);
                }
            }
            // Focused(bool),
            KeyboardInput { input, is_synthetic, .. } => {
                if input.state == ElementState::Pressed && !is_synthetic {
                    if let Some(vkey) = input.virtual_keycode {
                        self.start_key_event(widget, vkey, input.scancode);
                    }
                } else if input.state == ElementState::Released {
                    self.end_key_event(input.scancode);
                }
            }
            ModifiersChanged(state) => {
                self.mgr.modifiers = state;
            }
            CursorMoved {
                position,
                ..
            } => {
                let coord = position.into();

                // Update hovered widget
                let cur_id = widget.find_id(coord);
                let delta = coord - self.mgr.last_mouse_coord;
                self.set_hover(widget, cur_id);

                if let Some(grab) = self.mouse_grab() {
                    if grab.mode == GrabMode::Grab {
                        let source = PressSource::Mouse(grab.button);
                        let event = Event::PressMove { source, cur_id, coord, delta };
                        self.send_event(widget, grab.start_id, event);
                    } else if let Some(pan) = self.mgr.pan_grab.get_mut(grab.pan_grab.0 as usize) {
                        pan.coords[grab.pan_grab.1 as usize].1 = coord;
                    }
                } else if let Some(id) = self.mgr.press_focus {
                    // Use a fake button!
                    let source = PressSource::Mouse(MouseButton::Other(0));
                    let event = Event::PressMove { source, cur_id, coord, delta };
                    self.send_event(widget, id, event);
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
                let event = Event::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) =>
                        ScrollDelta::PixelDelta(Coord::from_logical(pos, self.mgr.dpi_factor)),
                });
                if let Some(id) = self.mgr.hover {
                    self.send_event(widget, id, event);
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
                            debug_assert_eq!(state, ElementState::Released);
                            let event = Event::PressEnd {
                                source,
                                end_id: self.mgr.hover,
                                coord,
                            };
                            self.send_event(widget, grab.start_id, event);
                        }
                        // Pan events do not receive Start/End notifications
                        _ => (),
                    };

                    if state == ElementState::Released {
                        self.end_mouse_grab(button);
                    }
                } else if let Some(start_id) = self.mgr.hover {
                    // No mouse grab but have a hover target
                    if state == ElementState::Pressed {
                        self.send_press_start(widget, source, start_id, coord);
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
                        if let Some(start_id) = widget.find_id(coord) {
                            self.send_press_start(widget, source, start_id, coord);
                        }
                    }
                    TouchPhase::Moved => {
                        let cur_id = widget.find_id(coord);

                        let mut r = None;
                        let mut pan_grab = None;
                        if let Some(grab) = self.get_touch(touch.id) {
                            if grab.mode == GrabMode::Grab {
                                let id = grab.start_id;
                                let event = Event::PressMove {
                                    source,
                                    cur_id,
                                    coord,
                                    delta: coord - grab.coord,
                                };
                                // Only when 'depressed' status changes:
                                let redraw = grab.cur_id != cur_id &&
                                    (grab.cur_id == Some(grab.start_id) || cur_id == Some(grab.start_id));

                                grab.cur_id = cur_id;
                                grab.coord = coord;

                                r = Some((id, event, redraw));
                            } else {
                                pan_grab = Some(grab.pan_grab);
                            }
                        }

                        if let Some((id, event, redraw)) = r {
                            if redraw {
                                self.send_action(TkAction::Redraw);
                            }
                            self.send_event(widget, id, event);
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
                                let event = Event::PressEnd {
                                    source,
                                    end_id: grab.cur_id,
                                    coord,
                                };
                                if let Some(cur_id) = grab.cur_id {
                                    self.redraw(cur_id);
                                }
                                self.send_event(widget, grab.start_id, event);
                            } else {
                                self.mgr.remove_pan_grab(grab.pan_grab);
                            }
                        }
                    }
                    TouchPhase::Cancelled => {
                        if let Some(grab) = self.remove_touch(touch.id) {
                            let event = Event::PressEnd {
                                source,
                                end_id: None,
                                coord,
                            };
                            if let Some(cur_id) = grab.cur_id {
                                self.redraw(cur_id);
                            }
                            self.send_event(widget, grab.start_id, event);
                        }
                    }
                }
            }
            // HiDpiFactorChanged(factor) [handled by toolkit]
            _ => (),
        }
    }
}
