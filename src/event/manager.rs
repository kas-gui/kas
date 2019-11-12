// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use std::collections::HashMap;

use super::*;
use crate::{TkWindow, Widget, WidgetId};

/// Window event manager
///
/// Event handling requires some state on the window; this struct provides that.
#[derive(Clone, Debug)]
pub struct ManagerData {
    dpi_factor: f64,
    grab_focus: Option<WidgetId>,
    key_focus: Option<WidgetId>,
    hover: Option<WidgetId>,
    click_start: Option<WidgetId>,
    touch_events: Vec<(u64, WidgetId, WidgetId)>,
    accel_keys: HashMap<VirtualKeyCode, WidgetId>,
}

impl ManagerData {
    /// Construct an event manager per-window data struct
    ///
    /// (For toolkit use.)
    ///
    /// The DPI factor may be required for event coordinate translation.
    #[inline]
    pub fn new(dpi_factor: f64) -> Self {
        ManagerData {
            dpi_factor,
            grab_focus: None,
            key_focus: None,
            hover: None,
            click_start: None,
            touch_events: Vec::with_capacity(10),
            accel_keys: HashMap::new(),
        }
    }

    /// Configure event manager for a widget tree.
    ///
    /// This should be called by the toolkit on the widget tree when the window
    /// is created (before or after resizing).
    pub fn configure(&mut self, widget: &mut dyn Widget) {
        let mut id = WidgetId::FIRST;
        self.accel_keys.clear();
        widget.walk_mut(&mut |widget| {
            widget.core_data_mut().id = id;
            for key in widget.core_data().keys() {
                self.accel_keys.insert(key, id);
            }
            id = id.next();
        });
    }

    /// Set the DPI factor. Must be updated for correct event translation by
    /// [`Manager::handle_winit`].
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

    /// Get whether this widget has a keyboard grab
    #[inline]
    pub fn key_grab(&self, w_id: WidgetId) -> bool {
        self.grab_focus == Some(w_id)
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
        for start in &self.touch_events {
            if start.2 == w_id {
                return true;
            }
        }
        false
    }

    /// Check whether the given widget is visually depressed
    #[inline]
    pub fn is_depressed(&self, w_id: WidgetId) -> bool {
        if self.click_start == Some(w_id) && self.hover == Some(w_id) {
            return true;
        }
        for start in &self.touch_events {
            if start.1 == w_id && start.2 == w_id {
                return true;
            }
        }
        false
    }

    fn set_hover(&mut self, w_id: Option<WidgetId>) -> bool {
        if self.hover != w_id {
            self.hover = w_id;
            return true;
        }
        false
    }
    fn set_click_start(&mut self, w_id: Option<WidgetId>) -> bool {
        if self.click_start != w_id {
            self.click_start = w_id;
            return true;
        }
        false
    }

    fn start_touch(&mut self, id: u64, w_id: WidgetId) -> bool {
        assert!(self.clear_touch(id) == false);
        self.touch_events.push((id, w_id, w_id));
        true
    }
    fn touch_move(&mut self, id: u64, w_id: WidgetId) -> bool {
        for start in &mut self.touch_events {
            if start.0 == id {
                start.2 = w_id;
                return true;
            }
        }
        assert!(false);
        false
    }
    fn touch_start(&self, id: u64) -> Option<WidgetId> {
        for start in &self.touch_events {
            if start.0 == id {
                return Some(start.1);
            }
        }
        None
    }
    fn clear_touch(&mut self, id: u64) -> bool {
        for (i, start) in self.touch_events.iter().enumerate() {
            if start.0 == id {
                self.touch_events.remove(i);
                return true;
            }
        }
        false
    }

    #[cfg(feature = "winit")]
    fn next_key_focus(&mut self, widget: &mut dyn Widget) -> bool {
        let start = self.key_focus;
        let mut id = start.map(|id| id.next()).unwrap_or(WidgetId::FIRST);
        let end = widget.id();
        while id <= end {
            if widget
                .get_by_id(id)
                .map(|w| w.class().allow_focus())
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

    pub(crate) fn set_grab(&mut self, id: WidgetId) -> bool {
        self.grab_focus = Some(id);
        self.key_focus = Some(id);
        true
    }
}

/// An interface for managing per-widget events
pub struct Manager;

impl Manager {
    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    // TODO: use widget.handle() return value?
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(widget: &mut W, tk: &mut dyn TkWindow, event: winit::event::WindowEvent)
    where
        W: Widget + Handler<Msg = EmptyMsg> + ?Sized,
    {
        use crate::TkAction;
        use winit::event::{TouchPhase, WindowEvent::*};
        // TODO: bind tk.data()
        match event {
            // Resized(size) [handled by toolkit]
            // Moved(position)
            CloseRequested => {
                tk.send_action(TkAction::Close);
            }
            // Destroyed
            // DroppedFile(PathBuf),
            // HoveredFile(PathBuf),
            // HoveredFileCancelled,
            ReceivedCharacter(c) if c != '\u{1b}' /* escape */ => {
                if let Some(id) = tk.data().grab_focus {
                    let ev = EventChild::Action(Action::ReceivedCharacter(c));
                    widget.handle(tk, Event::ToChild(id, ev));
                }
            }
            // Focused(bool),
            KeyboardInput { input, .. } => {
                match (input.state, input.virtual_keycode) {
                    (ElementState::Pressed, Some(vkey)) => match vkey {
                        VirtualKeyCode::Tab if tk.data().grab_focus.is_none() => {
                            tk.update_data(&mut |data| data.next_key_focus(widget.as_widget_mut()));
                        }
                        VirtualKeyCode::Return | VirtualKeyCode::NumpadEnter if tk.data().grab_focus.is_none() => {
                            if let Some(id) = tk.data().key_focus {
                                let ev = EventChild::Action(Action::Activate);
                                widget.handle(tk, Event::ToChild(id, ev));
                            }
                        }
                        VirtualKeyCode::Escape => {
                            tk.update_data(&mut |data| {
                                if data.grab_focus.is_some() {
                                    data.grab_focus = None;
                                    true
                                } else if data.key_focus.is_some() {
                                    data.key_focus = None;
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                        vkey @ _ if tk.data().grab_focus.is_none() => {
                            if let Some(id) = tk.data().accel_keys.get(&vkey).cloned() {
                                let ev = EventChild::Action(Action::Activate);
                                widget.handle(tk, Event::ToChild(id, ev));
                            }
                        }
                        _ /* implies grab_focus.is_some() */ => (),
                    },
                    _ => (),
                }
            }
            CursorMoved {
                position,
                modifiers,
                ..
            } => {
                let coord = position.to_physical(tk.data().dpi_factor).into();
                let ev = EventCoord::CursorMoved { modifiers };
                widget.handle(tk, Event::ToCoord(coord, ev));
            }
            // CursorEntered { .. },
            CursorLeft { .. } => {
                tk.update_data(&mut |data| data.set_hover(None));
            }
            // MouseWheel { delta: MouseScrollDelta, phase: TouchPhase, modifiers: ModifiersState, .. },
            MouseInput {
                state,
                button,
                modifiers,
                ..
            } => {
                let ev = EventChild::MouseInput {
                    state,
                    button,
                    modifiers,
                };
                tk.update_data(&mut |data| {
                    if data.grab_focus.is_some() && data.grab_focus != data.hover {
                        data.grab_focus = None;
                        true
                    } else {
                        false
                    }
                });
                if let Some(id) = tk.data().hover {
                    widget.handle(tk, Event::ToChild(id, ev));
                } else {
                    // This happens for example on click-release when the
                    // cursor is no longer over the window.
                    if button == MouseButton::Left && state == ElementState::Released {
                        tk.update_data(&mut |data| data.set_click_start(None));
                    }
                }
            }
            // TouchpadPressure { pressure: f32, stage: i64, },
            // AxisMotion { axis: AxisId, value: f64, },
            // RedrawRequested [handled by toolkit]
            Touch(touch) => {
                let coord = touch.location.to_physical(tk.data().dpi_factor).into();
                match touch.phase {
                    TouchPhase::Started => {
                        let ev = EventCoord::TouchStart(touch.id);
                        widget.handle(tk, Event::ToCoord(coord, ev));
                    }
                    TouchPhase::Moved => {
                        let ev = EventCoord::TouchMove(touch.id);
                        widget.handle(tk, Event::ToCoord(coord, ev));
                    }
                    TouchPhase::Ended => {
                        tk.update_data(&mut |data| {
                            let r = data.grab_focus.is_some();
                            data.grab_focus = None;
                            r
                        });
                        let ev = EventCoord::TouchEnd(touch.id);
                        widget.handle(tk, Event::ToCoord(coord, ev));
                    }
                    TouchPhase::Cancelled => {
                        tk.update_data(&mut |data| data.clear_touch(touch.id));
                    }
                }
            }
            // HiDpiFactorChanged(factor) [handled by toolkit]
            _ => {
                // println!("Unhandled window event: {:?}", event);
            }
        }
    }

    /// Generic handler for low-level events
    pub fn handle_generic<W>(
        widget: &mut W,
        tk: &mut dyn TkWindow,
        event: Event,
    ) -> <W as Handler>::Msg
    where
        W: Handler + ?Sized,
    {
        let w_id = widget.id();
        match event {
            Event::ToChild(_, ev) => match ev {
                EventChild::Action(action) => widget.handle_action(tk, action),
                EventChild::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                tk.update_data(&mut |data| data.set_click_start(Some(w_id)));
                                EmptyMsg.into()
                            }
                            ElementState::Released => {
                                let r = if tk.data().click_start == Some(w_id) {
                                    widget.handle_action(tk, Action::Activate)
                                } else {
                                    EmptyMsg.into()
                                };
                                tk.update_data(&mut |data| data.set_click_start(None));
                                r
                            }
                        }
                    } else {
                        EmptyMsg.into()
                    }
                }
            },
            Event::ToCoord(_, ev) => {
                match ev {
                    EventCoord::CursorMoved { .. } => {
                        // We can assume the pointer is over this widget
                        tk.update_data(&mut |data| data.set_hover(Some(w_id)));
                        EmptyMsg.into()
                    }
                    EventCoord::TouchStart(id) => {
                        tk.update_data(&mut |data| data.start_touch(id, w_id));
                        EmptyMsg.into()
                    }
                    EventCoord::TouchMove(id) => {
                        tk.update_data(&mut |data| data.touch_move(id, w_id));
                        EmptyMsg.into()
                    }
                    EventCoord::TouchEnd(id) => {
                        let r = if tk.data().touch_start(id) == Some(w_id) {
                            widget.handle_action(tk, Action::Activate)
                        } else {
                            EmptyMsg.into()
                        };
                        tk.update_data(&mut |data| data.clear_touch(id));
                        r
                    }
                }
            }
        }
    }
}
