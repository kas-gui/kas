// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use super::*;
use crate::{TkWindow, WidgetId};

/// Window event manager
///
/// Event handling requires some state on the window; this struct provides that.
#[derive(Clone, Debug)]
pub struct ManagerData {
    dpi_factor: f64,
    hover: Option<WidgetId>,
    click_start: Option<WidgetId>,
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
            hover: None,
            click_start: None,
        }
    }

    /// Set the DPI factor. Must be updated for correct event translation by
    /// [`Manager::handle_winit`].
    #[inline]
    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        self.dpi_factor = dpi_factor;
    }

    /// Get the widget under the mouse
    #[inline]
    pub fn mouse_hover(&self) -> Option<WidgetId> {
        self.hover
    }

    /// Get the widget depressed by the mouse
    ///
    /// Note that this may be a different widget than that given by
    /// [`ManagerData::mouse_hover`] since a widget counts as "depressed" by the mouse until
    /// a click action on that widget is either completed or cancelled. Visually
    /// a widget may only appear depressed if both point to the same widget.
    #[inline]
    pub fn mouse_depress(&self) -> Option<WidgetId> {
        self.click_start
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
}

/// An interface for managing per-widget events
pub struct Manager;

impl Manager {
    /// Handle a winit `WindowEvent`.
    ///
    /// Note that some event types are not *does not* handled, since for these
    /// events the toolkit must take direct action anyway:
    /// `Resized(size)`, `RedrawRequested`, `HiDpiFactorChanged(factor)`.
    #[cfg(feature = "winit")]
    pub fn handle_winit<W>(widget: &mut W, tk: &mut dyn TkWindow, event: WindowEvent)
    where
        W: Handler + ?Sized,
    {
        use crate::TkAction;
        use WindowEvent::*;
        match event {
            CloseRequested => {
                tk.send_action(TkAction::Close);
            }
            CursorMoved {
                device_id,
                position,
                modifiers,
            } => {
                let coord = position.to_physical(tk.data().dpi_factor).into();
                let ev = EventCoord::CursorMoved {
                    device_id,
                    modifiers,
                };
                widget.handle(tk, Event::ToCoord(coord, ev));
            }
            CursorLeft { .. } => {
                tk.update_data(&|data| data.set_hover(None));
            }
            MouseInput {
                device_id,
                state,
                button,
                modifiers,
            } => {
                let ev = EventChild::MouseInput {
                    device_id,
                    state,
                    button,
                    modifiers,
                };
                if let Some(id) = tk.data().hover {
                    widget.handle(tk, Event::ToChild(id, ev));
                } else {
                    // This happens for example on click-release when the
                    // cursor is no longer over the window.
                    if button == MouseButton::Left && state == ElementState::Released {
                        tk.update_data(&|data| data.set_click_start(None));
                    }
                }
            }
            _ => {
                // println!("Unhandled window event: {:?}", event);
            }
        };
    }

    /// Generic handler for low-level events
    pub fn handle_generic<W>(
        widget: &mut W,
        tk: &mut dyn TkWindow,
        event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        let w_id = Some(widget.id());
        match event {
            Event::ToChild(_, ev) => match ev {
                EventChild::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                tk.update_data(&|data| data.set_click_start(w_id));
                                Response::None
                            }
                            ElementState::Released => {
                                let r = if tk.data().click_start == w_id {
                                    widget.handle_action(tk, Action::Activate)
                                } else {
                                    Response::None
                                };
                                tk.update_data(&|data| data.set_click_start(None));
                                r
                            }
                        }
                    } else {
                        Response::None
                    }
                }
            },
            Event::ToCoord(_, ev) => {
                match ev {
                    EventCoord::CursorMoved { .. } => {
                        // We can assume the pointer is over this widget
                        tk.update_data(&|data| data.set_hover(w_id));
                        Response::None
                    }
                }
            }
        }
    }
}
