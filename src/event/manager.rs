// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event manager

use super::*;
use crate::TkWindow;

/// Window event manager
///
/// Event handling requires some state on the window; this struct provides that.
pub struct Manager;

impl Manager {
    /// Generic handler for low-level events
    pub fn handle_generic<W>(
        widget: &mut W,
        tk: &mut dyn TkWindow,
        event: Event,
    ) -> Response<<W as Handler>::Msg>
    where
        W: Handler + ?Sized,
    {
        let self_id = Some(widget.id());
        match event {
            Event::ToChild(_, ev) => match ev {
                EventChild::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                tk.set_click_start(self_id);
                                Response::None
                            }
                            ElementState::Released => {
                                let r = if tk.click_start() == self_id {
                                    widget.handle_action(tk, Action::Activate)
                                } else {
                                    Response::None
                                };
                                tk.set_click_start(None);
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
                        tk.set_hover(self_id);
                        Response::None
                    }
                }
            }
        }
    }
}
