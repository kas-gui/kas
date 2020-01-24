// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use std::collections::{hash_map::Entry, HashMap};

use super::*;
use crate::geom::Coord;
use crate::{TkAction, TkWindow, Widget, WidgetId, WindowId};

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
    pub key_focus: bool,
    /// "Character focus" implies this widget is ready to receive text input
    /// (e.g. typing into an input field).
    ///
    /// If true, this likely implies `key_focus` is also true.
    pub char_focus: bool,
}

impl HighlightState {
    /// True if any part of the state is true
    #[inline]
    pub fn any(self) -> bool {
        self.hover || self.depress || self.key_focus || self.char_focus
    }
}

#[derive(Clone, Debug)]
struct PressEvent {
    start_id: WidgetId,
    cur_id: WidgetId,
    last_coord: Coord,
}

/// Window event manager
///
/// Encapsulation of per-window event state plus supporting methods.
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
#[derive(Clone, Debug)]
pub struct ManagerState {
    dpi_factor: f64,
    char_focus: Option<WidgetId>,
    key_focus: Option<WidgetId>,
    hover: Option<WidgetId>,
    key_events: Vec<(u32, WidgetId)>,
    last_mouse_coord: Coord,
    mouse_grab: Option<(WidgetId, MouseButton)>,
    // TODO: would a VecMap be faster?
    touch_grab: HashMap<u64, PressEvent>,
    accel_keys: HashMap<VirtualKeyCode, WidgetId>,
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
            key_focus: None,
            hover: None,
            key_events: Vec::with_capacity(4),
            last_mouse_coord: Coord::ZERO,
            mouse_grab: None,
            touch_grab: HashMap::new(),
            accel_keys: HashMap::new(),
        }
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    pub fn configure<W>(&mut self, tkw: &mut dyn TkWindow, widget: &mut W)
    where
        W: Widget + Handler<Msg = VoidMsg> + ?Sized,
    {
        // Re-assigning WidgetIds might invalidate state; to avoid this we map
        // existing ids to new ids
        let mut map = HashMap::new();
        let mut id = WidgetId::FIRST;

        self.accel_keys.clear();
        let addr = Address::Coord(self.last_mouse_coord);
        let mut mgr = self.manager(tkw);
        widget.walk_mut(&mut |widget| {
            map.insert(widget.id(), id);
            widget.configure(id, &mut mgr);
            id = id.next();
        });

        self.hover = match widget.handle(&mut mgr, addr, Event::Identify) {
            Response::Identify(id) => Some(id),
            _ => None,
        };

        self.char_focus = self.char_focus.and_then(|id| map.get(&id).cloned());
        self.key_focus = self.key_focus.and_then(|id| map.get(&id).cloned());
        for event in &mut self.key_events {
            event.1 = map.get(&event.1).cloned().unwrap();
        }

        self.mouse_grab = self
            .mouse_grab
            .and_then(|(id, b)| map.get(&id).map(|id| (*id, b)));
        for event in &mut self.touch_grab {
            event.1.start_id = map.get(&event.1.start_id).cloned().unwrap();
            event.1.cur_id = map.get(&event.1.cur_id).cloned().unwrap();
        }
    }

    /// Set the DPI factor. Must be updated for correct event translation by
    /// [`Manager::handle_winit`].
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

    /// Construct a [`Manager`] referring to this state
    #[inline]
    pub fn manager<'a>(&'a mut self, tkw: &'a mut dyn TkWindow) -> Manager<'a> {
        Manager {
            action: TkAction::None,
            mgr: self,
            tkw,
        }
    }
}

/// Manager of event-handling and toolkit actions
pub struct Manager<'a> {
    action: TkAction,
    mgr: &'a mut ManagerState,
    tkw: &'a mut dyn TkWindow,
}

/// Public API (around toolkit functionality)
impl<'a> Manager<'a> {
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

    /// Attempt to get clipboard contents
    ///
    /// In case of failure, paste actions will simply fail. The implementation
    /// may wish to log an appropriate warning message.
    #[inline]
    pub fn get_clipboard(&mut self) -> Option<String> {
        self.tkw.get_clipboard()
    }

    /// Attempt to set clipboard contents
    #[inline]
    pub fn set_clipboard(&mut self, content: String) {
        self.tkw.set_clipboard(content)
    }
}

/// Public API (around event manager state)
impl<'a> Manager<'a> {
    /// Get the complete highlight state
    pub fn highlight_state(&self, w_id: WidgetId) -> HighlightState {
        HighlightState {
            hover: self.is_hovered(w_id),
            depress: self.is_depressed(w_id),
            key_focus: self.key_focus(w_id),
            char_focus: self.char_focus(w_id),
        }
    }

    /// Get whether this widget has a grab on character input
    #[inline]
    pub fn char_focus(&self, w_id: WidgetId) -> bool {
        self.mgr.char_focus == Some(w_id)
    }

    /// Get whether this widget has keyboard focus
    #[inline]
    pub fn key_focus(&self, w_id: WidgetId) -> bool {
        self.mgr.key_focus == Some(w_id)
    }

    /// Get whether the widget is under the mouse or finger
    #[inline]
    pub fn is_hovered(&self, w_id: WidgetId) -> bool {
        if self.mgr.hover == Some(w_id) {
            return true;
        }
        for touch in self.mgr.touch_grab.values() {
            if touch.cur_id == w_id {
                return true;
            }
        }
        false
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: WidgetId) -> bool {
        for (_, id) in &self.mgr.key_events {
            if *id == w_id {
                return true;
            }
        }
        if let Some(grab) = self.mgr.mouse_grab {
            if grab.0 == w_id && self.mgr.hover == Some(w_id) {
                return true;
            }
        }
        for touch in self.mgr.touch_grab.values() {
            if touch.start_id == w_id && touch.cur_id == w_id {
                return true;
            }
        }
        false
    }

    /// Adds an accelerator key for a widget
    ///
    /// If this key is pressed when the window has focus and no widget has a
    /// key-grab, the given widget will receive an [`Action::Activate`] event.
    #[inline]
    pub fn add_accel_key(&mut self, key: VirtualKeyCode, id: WidgetId) {
        self.mgr.accel_keys.insert(key, id);
    }

    /// Request character-input focus
    ///
    /// If successful, [`Action::ReceivedCharacter`] events are sent to this
    /// widget when character data is received.
    ///
    /// Currently, this method always succeeds.
    pub fn request_char_focus(&mut self, id: WidgetId) {
        if self.mgr.key_focus.is_some() {
            self.mgr.key_focus = Some(id);
        }
        self.mgr.char_focus = Some(id);
        self.redraw(id);
    }

    /// Request a mouse grab on the given `source`
    ///
    /// If successful, corresponding move/end events will be forwarded to the
    /// given `w_id`. The grab automatically ends after the end event. Since
    /// these events are *requested*, the widget should consume them even if
    /// e.g. the move events are not needed (although in practice this only
    /// affects parents intercepting [`Response::Unhandled`] events).
    ///
    /// This method normally succeeds, but fails when
    /// multiple widgets attempt a grab the same press source simultaneously
    /// (only the first grab is successful).
    ///
    /// This method automatically cancels any active char grab
    /// and updates keyboard navigation focus.
    pub fn request_press_grab(&mut self, source: PressSource, widget: &dyn Widget, coord: Coord) {
        let w_id = widget.id();
        match source {
            PressSource::Mouse(button) => {
                if self.mgr.mouse_grab.is_none() {
                    self.mgr.mouse_grab = Some((w_id, button));
                } else {
                    return;
                }
            }
            PressSource::Touch(touch_id) => match self.mgr.touch_grab.entry(touch_id) {
                Entry::Occupied(_) => return,
                Entry::Vacant(v) => {
                    v.insert(PressEvent {
                        start_id: w_id,
                        cur_id: w_id,
                        last_coord: coord,
                    });
                }
            },
        }

        if widget.allow_focus() {
            if self.mgr.key_focus.is_some() {
                self.mgr.key_focus = Some(w_id);
            }
            self.mgr.char_focus = None;
        }

        self.redraw(w_id);
    }
}

/// Internal methods
impl<'a> Manager<'a> {
    #[cfg(feature = "winit")]
    fn set_hover(&mut self, w_id: Option<WidgetId>) {
        if self.mgr.hover != w_id {
            self.mgr.hover = w_id;
            self.send_action(TkAction::Redraw);
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
    fn mouse_grab(&self) -> Option<(WidgetId, MouseButton)> {
        self.mgr.mouse_grab
    }

    #[cfg(feature = "winit")]
    fn end_mouse_grab(&mut self, button: MouseButton) {
        if let Some(grab) = self.mgr.mouse_grab {
            if grab.1 == button {
                self.mgr.mouse_grab = None;
                self.redraw(grab.0);
            }
        }
    }

    #[cfg(feature = "winit")]
    fn touch_grab(&self, touch_id: u64) -> Option<PressEvent> {
        self.mgr.touch_grab.get(&touch_id).cloned()
    }

    #[cfg(feature = "winit")]
    fn update_touch_coord(&mut self, touch_id: u64, coord: Coord) {
        if let Some(v) = self.mgr.touch_grab.get_mut(&touch_id) {
            v.last_coord = coord;
            // TODO: update cur_id (currently not calculated)
            // self.redraw(v.cur_id);
        }
    }

    #[cfg(feature = "winit")]
    fn end_touch_grab(&mut self, touch_id: u64) {
        if let Some(grab) = self.mgr.touch_grab.remove(&touch_id) {
            self.redraw(grab.cur_id);
        }
    }

    #[cfg(feature = "winit")]
    fn next_key_focus(&mut self, widget: &mut dyn Widget) {
        let mut id = self.mgr.key_focus.unwrap_or(WidgetId::FIRST);
        let end = widget.id();
        loop {
            id = id.next();
            if id >= end {
                return self.unset_key_focus();
            }

            if widget
                .get_by_id(id)
                .map(|w| w.allow_focus())
                .unwrap_or(false)
            {
                self.send_action(TkAction::Redraw);
                self.mgr.key_focus = Some(id);
                return;
            }
        }
    }

    #[cfg(feature = "winit")]
    fn unset_key_focus(&mut self) {
        if let Some(id) = self.mgr.key_focus {
            self.redraw(id);
        }
        self.mgr.key_focus = None;
    }
}

/// Toolkit API
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
impl<'a> Manager<'a> {
    /// Extract the [`TkAction`].
    pub fn unwrap_action(&mut self) -> TkAction {
        self.action
    }

    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(mut self, widget: &mut W, event: winit::event::WindowEvent) -> TkAction
    where
        W: Widget + Handler<Msg = VoidMsg> + ?Sized,
    {
        use log::trace;
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};
        trace!("Event: {:?}", event);

        let response = match event {
            // Resized(size) [handled by toolkit]
            // Moved(position)
            CloseRequested => {
                self.send_action(TkAction::Close);
                Response::None
            }
            // Destroyed
            // DroppedFile(PathBuf),
            // HoveredFile(PathBuf),
            // HoveredFileCancelled,
            ReceivedCharacter(c) if c != '\u{1b}' /* escape */ => {
                if let Some(id) = self.mgr.char_focus {
                    let ev = Event::Action(Action::ReceivedCharacter(c));
                    widget.handle(&mut self, Address::Id(id), ev)
                } else {
                    Response::None
                }
            }
            // Focused(bool),
            KeyboardInput { input, is_synthetic, .. } => {
                let char_focus = self.mgr.char_focus.is_some();
                match (input.scancode, input.state, input.virtual_keycode) {
                    (_, ElementState::Pressed, Some(vkey)) if char_focus && !is_synthetic => match vkey {
                        VirtualKeyCode::Escape => {
                            if let Some(id) = self.mgr.char_focus {
                                self.redraw(id);
                            }
                            self.mgr.char_focus = None;
                            Response::None
                        }
                        _ => Response::None,
                    },
                    (scancode, ElementState::Pressed, Some(vkey)) if !char_focus && !is_synthetic => match vkey {
                        VirtualKeyCode::Tab => {
                            self.next_key_focus(widget.as_widget_mut());
                            Response::None
                        }
                        VirtualKeyCode::Return | VirtualKeyCode::NumpadEnter => {
                            if let Some(id) = self.mgr.key_focus {
                                // Add to key_events for visual feedback
                                self.add_key_event(scancode, id);

                                let ev = Event::Action(Action::Activate);
                                widget.handle(&mut self, Address::Id(id), ev)
                            } else { Response::None }
                        }
                        VirtualKeyCode::Escape => {
                            self.unset_key_focus();
                            Response::None
                        }
                        vkey @ _ => {
                            if let Some(id) = self.mgr.accel_keys.get(&vkey).cloned() {
                                // Add to key_events for visual feedback
                                self.add_key_event(scancode, id);

                                let ev = Event::Action(Action::Activate);
                                widget.handle(&mut self, Address::Id(id), ev)
                            } else { Response::None }
                        }
                    },
                    (scancode, ElementState::Released, _) => {
                        self.remove_key_event(scancode);
                        Response::None
                    }
                    _ => Response::None,
                }
            }
            CursorMoved {
                position,
                ..
            } => {
                let coord = position.into();

                // Update hovered widget
                let w_id = match widget.handle(&mut self, Address::Coord(coord), Event::Identify) {
                    Response::Identify(w_id) => Some(w_id),
                    _ => None,
                };
                self.set_hover(w_id);

                let r = if let Some((grab_id, button)) = self.mouse_grab() {
                    let source = PressSource::Mouse(button);
                    let delta = coord - self.mgr.last_mouse_coord;
                    let ev = Event::PressMove { source, coord, delta };
                    widget.handle(&mut self, Address::Id(grab_id), ev)
                } else {
                    // We don't forward move events without a grab
                    Response::None
                };

                self.mgr.last_mouse_coord = coord;
                r
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                self.set_hover(None);
                Response::None
            }
            MouseWheel { delta, .. } => {
                let action = Action::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) =>
                        ScrollDelta::PixelDelta(Coord::from_logical(pos, self.mgr.dpi_factor)),
                });
                if let Some(id) = self.mgr.hover {
                    widget.handle(&mut self, Address::Id(id), Event::Action(action))
                } else {
                    Response::None
                }
            }
            MouseInput {
                state,
                button,
                ..
            } => {
                let coord = self.mgr.last_mouse_coord;
                let source = PressSource::Mouse(button);

                let r = if let Some((grab_id, _)) = self.mouse_grab() {
                    // Mouse grab active: send events there
                    let ev = match state {
                        // TODO: using grab_id as start_id is incorrect when
                        // multiple buttons are pressed simultaneously
                        ElementState::Pressed => Event::PressStart { source, coord },
                        ElementState::Released => Event::PressEnd {
                            source,
                            start_id: Some(grab_id),
                            end_id: self.mgr.hover,
                            coord,
                        },
                    };
                    widget.handle(&mut self, Address::Id(grab_id), ev)
                } else if let Some(id) = self.mgr.hover {
                    // No mouse grab, but we have a hover target
                    let ev = match state {
                        ElementState::Pressed => Event::PressStart { source, coord },
                        ElementState::Released => Event::PressEnd {
                            source,
                            start_id: None,
                            end_id: Some(id),
                            coord,
                        },
                    };
                    widget.handle(&mut self, Address::Id(id), ev)
                } else {
                    // This happens when there is no widget and on click-release
                    // when the cursor is no longer over the window.
                    Response::None
                };
                if state == ElementState::Released {
                    self.end_mouse_grab(button);
                }
                r
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            // RedrawRequested [handled by toolkit]
            Touch(touch) => {
                let source = PressSource::Touch(touch.id);
                let coord = touch.location.into();
                match touch.phase {
                    TouchPhase::Started => {
                        let ev = Event::PressStart { source, coord };
                        widget.handle(&mut self, Address::Coord(coord), ev)
                    }
                    TouchPhase::Moved => {
                        if let Some(PressEvent { start_id, last_coord, .. }) = self.touch_grab(touch.id) {
                            let action = Event::PressMove {
                                source,
                                coord,
                                delta: coord - last_coord,
                            };
                            let r = widget.handle(&mut self, Address::Id(start_id), action);
                            self.update_touch_coord(touch.id, coord);
                            r
                        } else {
                            Response::None
                        }
                    }
                    TouchPhase::Ended => {
                        if let Some(PressEvent { start_id, cur_id, .. }) = self.touch_grab(touch.id) {
                            let action = Event::PressEnd {
                                source,
                                start_id: Some(start_id),
                                end_id: Some(cur_id),
                                coord,
                            };
                            let r = widget.handle(&mut self, Address::Id(start_id), action);
                            self.end_touch_grab(touch.id);
                            r
                        } else {
                            let action = Event::PressEnd {
                                source,
                                start_id: None,
                                end_id: None,
                                coord,
                            };
                            widget.handle(&mut self, Address::Coord(coord), action)
                        }
                    }
                    TouchPhase::Cancelled => {
                        if let Some(PressEvent { start_id, .. }) = self.touch_grab(touch.id) {
                            let action = Event::PressEnd {
                                source,
                                start_id: Some(start_id),
                                end_id: None,
                                coord,
                            };
                            let r = widget.handle(&mut self, Address::Id(start_id), action);
                            self.end_touch_grab(touch.id);
                            r
                        } else {
                            Response::None
                        }
                    }
                }
            }
            // HiDpiFactorChanged(factor) [handled by toolkit]
            _ => Response::None,
        };

        match response {
            Response::None | Response::Identify(_) => (),
            Response::Unhandled(_) => {
                // we can safely ignore unhandled events here
            }
            Response::Msg(_) => unreachable!(),
        };

        self.unwrap_action()
    }
}
