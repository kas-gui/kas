// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::{Event, GrabMode, Manager, PressSource};
use crate::geom::{Coord, Offset};
use crate::WidgetId;

#[derive(Clone, Debug, PartialEq)]
enum TouchPhase {
    None,
    Start(u64, Coord), // id, coord
    Pan(u64),          // id
    Cursor(u64),       // id
}

impl Default for TouchPhase {
    fn default() -> Self {
        TouchPhase::None
    }
}

/// Handles text selection and panning from mouse and touch events
#[derive(Clone, Debug, Default)]
pub struct TextInput {
    touch_phase: TouchPhase,
}

/// Result of [`TextInput::handle`]
pub enum TextInputAction {
    /// No action (event consumed)
    None,
    /// Event not used
    Unhandled,
    /// Pan text using the given `delta`
    Pan(Offset),
    /// Update cursor and/or selection: `(coord, anchor, clear, repeats)`
    ///
    /// The cursor position should be moved to `coord`.
    ///
    /// If `anchor`, the anchor position (used for word and line selection mode)
    /// should be set to the new cursor position.
    ///
    /// If `clear`, the selection should be cleared (move selection position to
    /// edit position).
    ///
    /// If `repeats > 1`, [`SelectionHelper::expand`] should be called with
    /// this parameter to enable word/line selection mode.
    Cursor(Coord, bool, bool, u32),
}

impl TextInput {
    /// Handle input events
    ///
    /// Consumes the following events: `PressStart`, `PressMove`, `PressEnd`,
    /// `TimerUpdate`. May request press grabs and timer updates.
    pub fn handle(&mut self, mgr: &mut Manager, w_id: WidgetId, event: Event) -> TextInputAction {
        use TextInputAction as Action;
        match event {
            Event::PressStart { source, coord, .. } if source.is_primary() => {
                mgr.request_grab(w_id, source, coord, GrabMode::Grab, None);
                mgr.request_char_focus(w_id);
                match source {
                    PressSource::Touch(touch_id) => {
                        if self.touch_phase == TouchPhase::None {
                            self.touch_phase = TouchPhase::Start(touch_id, coord);
                            let delay = mgr.config().touch_text_sel_delay();
                            mgr.update_on_timer(delay, w_id, touch_id);
                        }
                        Action::None
                    }
                    PressSource::Mouse(..) if mgr.modifiers().ctrl() => {
                        // With Ctrl held, we scroll instead of moving the cursor
                        // (non-standard, but seems to work well)!
                        Action::None
                    }
                    PressSource::Mouse(_, repeats) => {
                        Action::Cursor(coord, true, !mgr.modifiers().shift(), repeats)
                    }
                }
            }
            Event::PressMove {
                source,
                coord,
                delta,
                ..
            } => {
                let ctrl = mgr.modifiers().ctrl();
                match source {
                    PressSource::Touch(touch_id) => match self.touch_phase {
                        TouchPhase::Start(id, ..) if id == touch_id => {
                            self.touch_phase = TouchPhase::Pan(id);
                            Action::Pan(delta)
                        }
                        TouchPhase::Pan(id) if id == touch_id => Action::Pan(delta),
                        TouchPhase::Cursor(id) if ctrl && id == touch_id => Action::Pan(delta),
                        _ => Action::Cursor(coord, false, false, 1),
                    },
                    PressSource::Mouse(..) if ctrl => Action::Pan(delta),
                    PressSource::Mouse(_, repeats) => Action::Cursor(coord, false, false, repeats),
                }
            }
            Event::PressEnd { source, .. } => {
                match self.touch_phase {
                    TouchPhase::Start(id, ..) | TouchPhase::Pan(id) | TouchPhase::Cursor(id)
                        if source == PressSource::Touch(id) =>
                    {
                        self.touch_phase = TouchPhase::None;
                    }
                    _ => (),
                }
                Action::None
            }
            Event::TimerUpdate(payload) => {
                match self.touch_phase {
                    TouchPhase::Start(touch_id, coord) if touch_id == payload => {
                        self.touch_phase = TouchPhase::Cursor(touch_id);
                        if mgr.modifiers().ctrl() {
                            Action::None
                        } else {
                            Action::Cursor(coord, false, !mgr.modifiers().shift(), 1)
                        }
                    }
                    // Note: if the TimerUpdate were from another requester it
                    // should technically be Unhandled, but it doesn't matter
                    // so long as other consumers match this first.
                    _ => Action::None,
                }
            }
            _ => Action::Unhandled,
        }
    }
}
