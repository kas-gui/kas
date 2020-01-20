// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use std::collections::{hash_map::Entry, HashMap};

use super::*;
use crate::geom::Coord;
use crate::{Widget, WidgetId};

/// Highlighting state of a widget
#[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
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
#[derive(Clone, Debug)]
pub struct Manager {
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

impl Manager {
    /// Construct an event manager per-window data struct
    ///
    /// The DPI factor may be required for event coordinate translation.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[inline]
    pub fn new(dpi_factor: f64) -> Self {
        Manager {
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
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    pub fn configure(&mut self, widget: &mut dyn Widget) {
        // Re-assigning WidgetIds might invalidate state; to avoid this we map
        // existing ids to new ids
        let mut map = HashMap::new();
        let mut id = WidgetId::FIRST;

        self.accel_keys.clear();
        widget.walk_mut(&mut |widget| {
            map.insert(widget.id(), id);
            widget.configure(id, self);
            id = id.next();
        });

        self.char_focus = self.char_focus.and_then(|id| map.get(&id).cloned());
        self.key_focus = self.key_focus.and_then(|id| map.get(&id).cloned());
        for event in &mut self.key_events {
            event.1 = map.get(&event.1).cloned().unwrap();
        }

        // TODO: this widget may no longer be under the mouse pointer!
        // We have addr = Address::Coord(self.last_mouse_coord), but cannot use
        // widget.handle(tk, addr, Event::Identify) because we don't have tk
        // (and the caller cannot construct this: ev_mgr is already borrowed).
        // Solution: add some other method to resolve a widget from a coord.
        self.hover = self.hover.and_then(|id| map.get(&id).cloned());

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
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

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
        self.char_focus == Some(w_id)
    }

    /// Get whether this widget has keyboard focus
    #[inline]
    pub fn key_focus(&self, w_id: WidgetId) -> bool {
        self.key_focus == Some(w_id)
    }

    /// Get whether the widget is under the mouse or finger
    #[inline]
    pub fn is_hovered(&self, w_id: WidgetId) -> bool {
        if self.hover == Some(w_id) {
            return true;
        }
        for touch in self.touch_grab.values() {
            if touch.cur_id == w_id {
                return true;
            }
        }
        false
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: WidgetId) -> bool {
        for (_, id) in &self.key_events {
            if *id == w_id {
                return true;
            }
        }
        if let Some(grab) = self.mouse_grab {
            if grab.0 == w_id && self.hover == Some(w_id) {
                return true;
            }
        }
        for touch in self.touch_grab.values() {
            if touch.start_id == w_id && touch.cur_id == w_id {
                return true;
            }
        }
        false
    }

    #[cfg(feature = "winit")]
    fn set_hover(&mut self, w_id: Option<WidgetId>) -> bool {
        if self.hover != w_id {
            self.hover = w_id;
            return true;
        }
        false
    }

    #[cfg(feature = "winit")]
    fn set_last_mouse_coord(&mut self, coord: Coord) -> bool {
        self.last_mouse_coord = coord;
        false
    }

    /// Adds an accelerator key for a widget
    ///
    /// If this key is pressed when the window has focus and no widget has a
    /// key-grab, the given widget will receive an [`Action::Activate`] event.
    #[inline]
    pub fn add_accel_key(&mut self, key: VirtualKeyCode, id: WidgetId) {
        self.accel_keys.insert(key, id);
    }

    /// Request a mouse grab on the given input source
    ///
    /// Also adjusts keyboard focus
    ///
    /// If successful, corresponding move/end events will be forwarded to the
    /// given `w_id`. The grab automatically ends after the end event. Since
    /// these events are *requested*, the widget should consume them even if
    /// e.g. the move events are not needed (although in practice this only
    /// affects parents intercepting [`Response::Unhandled`] events).
    ///
    /// In the case that multiple widgets attempt to grab the same source, only
    /// the first will be successful.
    pub fn request_press_grab(
        &mut self,
        source: PressSource,
        widget: &dyn Widget,
        coord: Coord,
    ) -> bool {
        let w_id = widget.id();
        if widget.allow_focus() {
            if self.key_focus.is_some() {
                self.key_focus = Some(w_id);
            }
            self.char_focus = None;
        }

        match source {
            PressSource::Mouse(button) => {
                if self.mouse_grab.is_none() {
                    self.mouse_grab = Some((w_id, button));
                    true
                } else {
                    false
                }
            }
            PressSource::Touch(touch_id) => match self.touch_grab.entry(touch_id) {
                Entry::Occupied(_) => false,
                Entry::Vacant(v) => {
                    v.insert(PressEvent {
                        start_id: w_id,
                        cur_id: w_id,
                        last_coord: coord,
                    });
                    true
                }
            },
        }
    }

    #[cfg(feature = "winit")]
    fn mouse_grab(&self) -> Option<(WidgetId, MouseButton)> {
        self.mouse_grab
    }

    #[cfg(feature = "winit")]
    fn end_mouse_grab(&mut self, button: MouseButton) -> bool {
        if self.mouse_grab.map(|g| g.1 == button).unwrap_or(false) {
            self.mouse_grab = None;
            true
        } else {
            false
        }
    }

    #[cfg(feature = "winit")]
    fn touch_grab(&self, touch_id: u64) -> Option<PressEvent> {
        self.touch_grab.get(&touch_id).cloned()
    }

    #[cfg(feature = "winit")]
    fn update_touch_coord(&mut self, touch_id: u64, coord: Coord) -> bool {
        if let Some(v) = self.touch_grab.get_mut(&touch_id) {
            v.last_coord = coord;
            true
        } else {
            false
        }
    }

    #[cfg(feature = "winit")]
    fn end_touch_grab(&mut self, touch_id: u64) -> bool {
        self.touch_grab.remove(&touch_id).is_some()
    }

    #[cfg(feature = "winit")]
    fn next_key_focus(&mut self, widget: &mut dyn Widget) -> bool {
        let start = self.key_focus;
        let mut id = start.map(|id| id.next()).unwrap_or(WidgetId::FIRST);
        let end = widget.id();
        while id <= end {
            if widget
                .get_by_id(id)
                .map(|w| w.allow_focus())
                .unwrap_or(false)
            {
                self.key_focus = Some(id);
                return start != Some(id);
            }
            id = id.next();
        }
        self.key_focus = None;
        start != None
    }

    pub(crate) fn set_char_focus(&mut self, id: WidgetId) -> bool {
        if self.key_focus.is_some() {
            self.key_focus = Some(id);
        }
        self.char_focus = Some(id);
        true
    }
}

impl Manager {
    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(
        widget: &mut W,
        tk: &mut dyn crate::TkWindow,
        event: winit::event::WindowEvent,
    ) where
        W: Widget + Handler<Msg = VoidMsg> + ?Sized,
    {
        use crate::TkAction;
        use log::trace;
        use winit::event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent::*};
        trace!("Event: {:?}", event);

        let response = match event {
            // Resized(size) [handled by toolkit]
            // Moved(position)
            CloseRequested => {
                tk.send_action(TkAction::Close);
                Response::None
            }
            // Destroyed
            // DroppedFile(PathBuf),
            // HoveredFile(PathBuf),
            // HoveredFileCancelled,
            ReceivedCharacter(c) if c != '\u{1b}' /* escape */ => {
                if let Some(id) = tk.data().char_focus {
                    let ev = Event::Action(Action::ReceivedCharacter(c));
                    widget.handle(tk, Address::Id(id), ev)
                } else {
                    Response::None
                }
            }
            // Focused(bool),
            KeyboardInput { input, .. } => {
                let char_focus = tk.data().char_focus.is_some();
                match (input.scancode, input.state, input.virtual_keycode) {
                    (_, ElementState::Pressed, Some(vkey)) if char_focus => match vkey {
                        VirtualKeyCode::Escape => {
                            tk.update_data(&mut |data| {
                                data.char_focus = None;
                                true
                            });
                            Response::None
                        }
                        _ => Response::None,
                    },
                    (scancode, ElementState::Pressed, Some(vkey)) if !char_focus => match vkey {
                        VirtualKeyCode::Tab => {
                            tk.update_data(&mut |data| data.next_key_focus(widget.as_widget_mut()));
                            Response::None
                        }
                        VirtualKeyCode::Return | VirtualKeyCode::NumpadEnter => {
                            if let Some(id) = tk.data().key_focus {
                                let ev = Event::Action(Action::Activate);
                                let r =  widget.handle(tk, Address::Id(id), ev);

                                // Add to key_events for visual feedback
                                tk.update_data(&mut |data| {
                                    for item in &data.key_events {
                                        if item.1 == id {
                                            return false;
                                        }
                                    }
                                    data.key_events.push((scancode, id));
                                    true
                                });
                                r
                            } else { Response::None }
                        }
                        VirtualKeyCode::Escape => {
                            tk.update_data(&mut |data| {
                                if data.key_focus.is_some() {
                                    data.key_focus = None;
                                    true
                                } else {
                                    false
                                }
                            });
                            Response::None
                        }
                        vkey @ _ => {
                            if let Some(id) = tk.data().accel_keys.get(&vkey).cloned() {
                                let ev = Event::Action(Action::Activate);
                                let r =  widget.handle(tk, Address::Id(id), ev);

                                tk.update_data(&mut |data| {
                                    for item in &data.key_events {
                                        if item.1 == id {
                                            return false;
                                        }
                                    }
                                    data.key_events.push((scancode, id));
                                    true
                                });
                                r
                            } else { Response::None }
                        }
                    },
                    (scancode, ElementState::Released, _) => {
                        tk.update_data(&mut |data| {
                            let r = 'outer: loop {
                                for (i, item) in data.key_events.iter().enumerate() {
                                    // We must match scancode not vkey since the
                                    // latter may have changed due to modifiers
                                    if item.0 == scancode {
                                        break 'outer i;
                                    }
                                }
                                return false;
                            };
                            data.key_events.remove(r);
                            true
                        });
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
                let w_id = match widget.handle(tk, Address::Coord(coord), Event::Identify) {
                    Response::Identify(w_id) => Some(w_id),
                    _ => None,
                };
                tk.update_data(&mut |data| data.set_hover(w_id));

                let r = if let Some((grab_id, button)) = tk.data().mouse_grab() {
                    let source = PressSource::Mouse(button);
                    let delta = coord - tk.data().last_mouse_coord;
                    let ev = Event::PressMove { source, coord, delta };
                    widget.handle(tk, Address::Id(grab_id), ev)
                } else {
                    // We don't forward move events without a grab
                    Response::None
                };

                tk.update_data(&mut |data| data.set_last_mouse_coord(coord));
                r
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                tk.update_data(&mut |data| data.set_hover(None));
                Response::None
            }
            MouseWheel { delta, .. } => {
                let action = Action::Scroll(match delta {
                    MouseScrollDelta::LineDelta(x, y) => ScrollDelta::LineDelta(x, y),
                    MouseScrollDelta::PixelDelta(pos) =>
                        ScrollDelta::PixelDelta(Coord::from_logical(pos, tk.data().dpi_factor)),
                });
                if let Some(id) = tk.data().hover {
                    widget.handle(tk, Address::Id(id), Event::Action(action))
                } else {
                    Response::None
                }
            }
            MouseInput {
                state,
                button,
                ..
            } => {
                let coord = tk.data().last_mouse_coord;
                let source = PressSource::Mouse(button);

                let r = if let Some((grab_id, _)) = tk.data().mouse_grab() {
                    // Mouse grab active: send events there
                    let ev = match state {
                        // TODO: using grab_id as start_id is incorrect when
                        // multiple buttons are pressed simultaneously
                        ElementState::Pressed => Event::PressStart { source, coord },
                        ElementState::Released => Event::PressEnd {
                            source,
                            start_id: Some(grab_id),
                            end_id: tk.data().hover,
                            coord,
                        },
                    };
                    widget.handle(tk, Address::Id(grab_id), ev)
                } else if let Some(id) = tk.data().hover {
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
                    widget.handle(tk, Address::Id(id), ev)
                } else {
                    // This happens when there is no widget and on click-release
                    // when the cursor is no longer over the window.
                    Response::None
                };
                if state == ElementState::Released {
                    tk.update_data(&mut |data| data.end_mouse_grab(button));
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
                        widget.handle(tk, Address::Coord(coord), ev)
                    }
                    TouchPhase::Moved => {
                        if let Some(PressEvent { start_id, last_coord, .. }) = tk.data().touch_grab(touch.id) {
                            let action = Event::PressMove {
                                source,
                                coord,
                                delta: coord - last_coord,
                            };
                            let r = widget.handle(tk, Address::Id(start_id), action);
                            tk.update_data(&mut |data| data.update_touch_coord(touch.id, coord));
                            r
                        } else {
                            Response::None
                        }
                    }
                    TouchPhase::Ended => {
                        if let Some(PressEvent { start_id, cur_id, .. }) = tk.data().touch_grab(touch.id) {
                            let action = Event::PressEnd {
                                source,
                                start_id: Some(start_id),
                                end_id: Some(cur_id),
                                coord,
                            };
                            let r = widget.handle(tk, Address::Id(start_id), action);
                            tk.update_data(&mut |data| data.end_touch_grab(touch.id));
                            r
                        } else {
                            let action = Event::PressEnd {
                                source,
                                start_id: None,
                                end_id: None,
                                coord,
                            };
                            widget.handle(tk, Address::Coord(coord), action)
                        }
                    }
                    TouchPhase::Cancelled => {
                        if let Some(PressEvent { start_id, .. }) = tk.data().touch_grab(touch.id) {
                            let action = Event::PressEnd {
                                source,
                                start_id: Some(start_id),
                                end_id: None,
                                coord,
                            };
                            let r = widget.handle(tk, Address::Id(start_id), action);
                            tk.update_data(&mut |data| data.end_touch_grab(touch.id));
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
    }
}
