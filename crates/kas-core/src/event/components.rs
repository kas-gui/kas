// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::ScrollDelta::{LineDelta, PixelDelta};
use super::{Command, CursorIcon, Event, EventMgr, PressSource, Response, Scroll};
use crate::cast::traits::*;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
#[allow(unused)]
use crate::text::SelectionHelper;
use crate::{TkAction, WidgetId};
use kas_macros::impl_default;
use std::time::{Duration, Instant};

const PAYLOAD_SELECT: u64 = 1 << 60;
const PAYLOAD_GLIDE: u64 = (1 << 60) + 1;
const GLIDE_POLL_MS: u64 = 3;

#[derive(Clone, Debug, PartialEq)]
enum Glide {
    None,
    Drag(u8, [(Instant, Offset); 4]),
    Glide(Instant, Vec2, Vec2),
}

impl Default for Glide {
    fn default() -> Self {
        Glide::None
    }
}

impl Glide {
    fn move_delta(&mut self, delta: Offset) {
        match self {
            Glide::Drag(next, samples) => {
                samples[*next as usize] = (Instant::now(), delta);
                *next = (*next + 1) % 4;
            }
            _ => {
                let x = (Instant::now(), delta);
                *self = Glide::Drag(1, [x; 4]);
            }
        }
    }

    fn opt_start(&mut self, timeout: Duration) -> bool {
        if let Glide::Drag(_, samples) = self {
            let now = Instant::now();
            let start = now - timeout;
            let mut delta = Offset::ZERO;
            let mut t0 = now;
            for (time, d) in samples {
                if *time >= start {
                    t0 = t0.min(*time);
                    delta += *d;
                }
            }
            let dur = now - t0;
            let v = Vec2::conv(delta) / dur.as_secs_f32();
            if dur >= Duration::from_millis(1) && v != Vec2::ZERO {
                *self = Glide::Glide(now, v, Vec2::ZERO);
                true
            } else {
                *self = Glide::None;
                false
            }
        } else {
            false
        }
    }

    fn step(&mut self, (decay_mul, decay_sub): (f32, f32)) -> Option<Offset> {
        if let Glide::Glide(start, v, rest) = self {
            let now = Instant::now();
            let dur = (now - *start).as_secs_f32();
            let d = *v * dur + *rest;
            let delta = Offset::conv_approx(d);
            let rest = d - Vec2::conv(delta);

            if v.max_abs_comp() >= 1.0 {
                let mut v = *v * decay_mul.powf(dur);
                v = v - v.abs().min(Vec2::splat(decay_sub * dur)) * v.sign();
                *self = Glide::Glide(now, v, rest);
                Some(delta)
            } else {
                *self = Glide::None;
                None
            }
        } else {
            None
        }
    }
}

/// Logic for a scroll region
///
/// This struct handles some scroll logic. It does not provide scrollbars.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollComponent {
    max_offset: Offset,
    offset: Offset,
    glide: Glide,
}

impl Default for ScrollComponent {
    #[inline]
    fn default() -> Self {
        ScrollComponent {
            max_offset: Offset::ZERO,
            offset: Offset::ZERO,
            glide: Glide::None,
        }
    }
}

impl ScrollComponent {
    /// Get the maximum offset
    ///
    /// Note: the minimum offset is always zero.
    #[inline]
    pub fn max_offset(&self) -> Offset {
        self.max_offset
    }

    /// Get the current offset
    ///
    /// To translate a coordinate from the outer region to a coordinate of the
    /// scrolled region, add this offset.
    #[inline]
    pub fn offset(&self) -> Offset {
        self.offset
    }

    /// Set sizes:
    ///
    /// -   `window_size`: size of scroll region on the outside
    /// -   `content_size`: size of scroll region on the inside (usually larger)
    ///
    /// Like [`Self::set_offset`] this generates a [`TkAction`] due to potential
    /// change in offset. In practice the caller will likely be performing all
    /// required updates regardless and the return value can be safely ignored.
    pub fn set_sizes(&mut self, window_size: Size, content_size: Size) -> TkAction {
        self.max_offset = Offset::conv(content_size) - Offset::conv(window_size);
        self.set_offset(self.offset)
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`TkAction::empty()`] if the offset is identical to the old offset,
    /// or [`TkAction::REGION_MOVED`] if the offset changes.
    pub fn set_offset(&mut self, offset: Offset) -> TkAction {
        let offset = offset.min(self.max_offset).max(Offset::ZERO);
        if offset == self.offset {
            TkAction::empty()
        } else {
            self.offset = offset;
            TkAction::REGION_MOVED
        }
    }

    /// Scroll to make the given `rect` visible
    ///
    /// Inputs and outputs:
    ///
    /// -   `rect`: the rect to focus in child's coordinate space
    /// -   `window_rect`: the rect of the scroll window
    /// -   returned `Rect`: the focus rect, adjusted for scroll offset; this
    ///     may be set via [`EventMgr::set_scroll`]
    /// -   returned `TkAction`: action to pass to the event manager
    pub fn focus_rect(&mut self, rect: Rect, window_rect: Rect) -> (Rect, TkAction) {
        let v = rect.pos - window_rect.pos;
        let off = Offset::conv(rect.size) - Offset::conv(window_rect.size);
        let offset = self.offset.max(v + off).min(v);
        let action = self.set_offset(offset);
        (rect - self.offset, action)
    }

    /// Handle a [`Scroll`] action
    pub fn scroll(&mut self, mgr: &mut EventMgr, window_rect: Rect, scroll: Scroll) {
        match scroll {
            Scroll::None | Scroll::Scrolled => (),
            Scroll::Offset(delta) => {
                let old_offset = self.offset;
                *mgr |= self.set_offset(old_offset - delta);
                mgr.set_scroll(match delta - old_offset + self.offset {
                    delta if delta == Offset::ZERO => Scroll::Scrolled,
                    delta => Scroll::Offset(delta),
                });
            }
            Scroll::Rect(rect) => {
                let (rect, action) = self.focus_rect(rect, window_rect);
                *mgr |= action;
                mgr.set_scroll(Scroll::Rect(rect));
            }
        }
    }

    fn scroll_by_delta(&mut self, mgr: &mut EventMgr, d: Offset) -> bool {
        let old_offset = self.offset;
        *mgr |= self.set_offset(old_offset - d);
        let delta = d - (old_offset - self.offset);
        mgr.set_scroll(if delta != Offset::ZERO {
            Scroll::Offset(delta)
        } else {
            Scroll::Scrolled
        });
        old_offset != self.offset
    }

    /// Use an event to scroll, if possible
    ///
    /// Consumes the following events: `Command`, `Scroll`, `PressStart`,
    /// `PressMove`, `PressEnd`, `TimerUpdate(pl)` where `pl == (1<<60) + 1`.
    /// May request timer updates.
    ///
    /// Implements scroll by Home/End, Page Up/Down and arrow keys, by mouse
    /// wheel and touchpad.
    ///
    /// `PressStart` is consumed only if the maximum scroll offset is non-zero
    /// and event configuration enables panning for this press `source` (may
    /// depend on modifiers), and if so grabs press events from this `source`.
    /// `PressMove` is used to scroll by the motion delta and to track speed;
    /// `PressEnd` initiates momentum-scrolling if the speed is high enough.
    ///
    /// Returns `(moved, response)`.
    pub fn scroll_by_event(
        &mut self,
        mgr: &mut EventMgr,
        event: Event,
        id: WidgetId,
        window_rect: Rect,
    ) -> (bool, Response) {
        let mut moved = false;
        match event {
            Event::Command(cmd) => {
                let offset = match cmd {
                    Command::Home => Offset::ZERO,
                    Command::End => self.max_offset,
                    cmd => {
                        let delta = match cmd {
                            Command::Left => LineDelta(-1.0, 0.0),
                            Command::Right => LineDelta(1.0, 0.0),
                            Command::Up => LineDelta(0.0, 1.0),
                            Command::Down => LineDelta(0.0, -1.0),
                            Command::PageUp => PixelDelta(Offset(0, window_rect.size.1 / 2)),
                            Command::PageDown => PixelDelta(Offset(0, -(window_rect.size.1 / 2))),
                            _ => return (false, Response::Unused),
                        };
                        let delta = match delta {
                            LineDelta(x, y) => mgr.config().scroll_distance((-x, y), None),
                            PixelDelta(d) => d,
                        };
                        self.offset - delta
                    }
                };
                let action = self.set_offset(offset);
                if !action.is_empty() {
                    moved = true;
                    *mgr |= action;
                }
                mgr.set_scroll(Scroll::Rect(window_rect));
            }
            Event::Scroll(delta) => {
                let delta = match delta {
                    LineDelta(x, y) => mgr.config().scroll_distance((-x, y), None),
                    PixelDelta(d) => d,
                };
                moved = self.scroll_by_delta(mgr, delta);
            }
            Event::PressStart { source, coord, .. }
                if self.max_offset != Offset::ZERO && mgr.config_enable_pan(source) =>
            {
                let icon = Some(CursorIcon::Grabbing);
                mgr.grab_press_unique(id, source, coord, icon);
            }
            Event::PressMove { delta, .. } => {
                self.glide.move_delta(delta);
                moved = self.scroll_by_delta(mgr, delta);
            }
            Event::PressEnd { .. } => {
                if self.glide.opt_start(mgr.config().scroll_flick_timeout()) {
                    mgr.request_update(id, PAYLOAD_GLIDE, Duration::new(0, 0), true);
                }
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let decay = mgr.config().scroll_flick_decay();
                if let Some(delta) = self.glide.step(decay) {
                    let action = self.set_offset(self.offset - delta);
                    if !action.is_empty() {
                        *mgr |= action;
                        moved = true;
                    }
                    if delta == Offset::ZERO || !action.is_empty() {
                        // Note: when FPS > pixels/sec, delta may be zero while
                        // still scrolling. Glide returns None when we're done,
                        // but we're also done if unable to scroll further.
                        let dur = Duration::from_millis(GLIDE_POLL_MS);
                        mgr.request_update(id, PAYLOAD_GLIDE, dur, true);
                        mgr.set_scroll(Scroll::Scrolled);
                    }
                }
            }
            _ => return (false, Response::Unused),
        }
        (moved, Response::Used)
    }
}

#[impl_default(TouchPhase::None)]
#[derive(Clone, Debug, PartialEq)]
enum TouchPhase {
    None,
    Start(u64, Coord), // id, coord
    Pan(u64),          // id
    Cursor(u64),       // id
}

/// Handles text selection and panning from mouse and touch events
#[derive(Clone, Debug, Default)]
pub struct TextInput {
    touch_phase: TouchPhase,
    glide: Glide,
}

/// Result of [`TextInput::handle`]
pub enum TextInputAction {
    /// No action (event consumed)
    None,
    /// Event not used
    Unused,
    /// Pan text using the given `delta`
    Pan(Offset),
    /// Keyboard focus should be requested (if not already active)
    ///
    /// This is also the case for variant `Cursor(_, true, _, _)` (i.e. if
    /// `anchor == true`).
    Focus,
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
    /// `TimerUpdate(pl)` where `pl == 1<<60 || pl == (1<<60)+1`.
    /// May request press grabs and timer updates.
    ///
    /// Implements scrolling and text selection behaviour, excluding handling of
    /// [`Event::Scroll`].
    pub fn handle(&mut self, mgr: &mut EventMgr, w_id: WidgetId, event: Event) -> TextInputAction {
        use TextInputAction as Action;
        match event {
            Event::PressStart { source, coord, .. } if source.is_primary() => {
                let (action, icon) = match source {
                    PressSource::Touch(touch_id) => {
                        self.touch_phase = TouchPhase::Start(touch_id, coord);
                        let delay = mgr.config().touch_select_delay();
                        mgr.request_update(w_id.clone(), PAYLOAD_SELECT, delay, false);
                        (Action::Focus, None)
                    }
                    PressSource::Mouse(..) if mgr.config_enable_mouse_text_pan() => {
                        (Action::Focus, Some(CursorIcon::Grabbing))
                    }
                    PressSource::Mouse(_, repeats) => (
                        Action::Cursor(coord, true, !mgr.modifiers().shift(), repeats),
                        None,
                    ),
                };
                mgr.grab_press_unique(w_id, source, coord, icon);
                action
            }
            Event::PressMove {
                source,
                coord,
                delta,
                ..
            } => {
                self.glide.move_delta(delta);
                match source {
                    PressSource::Touch(touch_id) => match self.touch_phase {
                        TouchPhase::Start(id, start_coord) if id == touch_id => {
                            let delta = coord - start_coord;
                            if mgr.config_test_pan_thresh(delta) {
                                self.touch_phase = TouchPhase::Pan(id);
                                Action::Pan(delta)
                            } else {
                                Action::None
                            }
                        }
                        TouchPhase::Pan(id) if id == touch_id => Action::Pan(delta),
                        _ => Action::Cursor(coord, false, false, 1),
                    },
                    PressSource::Mouse(..) if mgr.config_enable_mouse_text_pan() => {
                        Action::Pan(delta)
                    }
                    PressSource::Mouse(_, repeats) => Action::Cursor(coord, false, false, repeats),
                }
            }
            Event::PressEnd { source, .. } => {
                if self.glide.opt_start(mgr.config().scroll_flick_timeout())
                    && (matches!(source, PressSource::Touch(id) if self.touch_phase == TouchPhase::Pan(id))
                        || matches!(source, PressSource::Mouse(..) if mgr.config_enable_mouse_text_pan()))
                {
                    self.touch_phase = TouchPhase::None;
                    mgr.request_update(w_id, PAYLOAD_GLIDE, Duration::new(0, 0), true);
                }
                Action::None
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_SELECT => {
                match self.touch_phase {
                    TouchPhase::Start(touch_id, coord) => {
                        self.touch_phase = TouchPhase::Cursor(touch_id);
                        Action::Cursor(coord, true, !mgr.modifiers().shift(), 1)
                    }
                    // Note: if the TimerUpdate were from another requester it
                    // should technically be Unused, but it doesn't matter
                    // so long as other consumers match this first.
                    _ => Action::None,
                }
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let decay = mgr.config().scroll_flick_decay();
                if let Some(delta) = self.glide.step(decay) {
                    let dur = Duration::from_millis(GLIDE_POLL_MS);
                    mgr.request_update(w_id, PAYLOAD_GLIDE, dur, true);
                    Action::Pan(delta)
                } else {
                    Action::None
                }
            }
            _ => Action::Unused,
        }
    }
}
