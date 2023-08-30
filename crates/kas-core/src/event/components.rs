// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::ScrollDelta::{LineDelta, PixelDelta};
use super::{Command, CursorIcon, Event, EventCx, PressSource, Response, Scroll};
use crate::cast::traits::*;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
#[allow(unused)] use crate::text::SelectionHelper;
use crate::{Action, WidgetId};
use kas_macros::impl_default;
use std::time::{Duration, Instant};

const PAYLOAD_SELECT: u64 = 1 << 60;
const PAYLOAD_GLIDE: u64 = (1 << 60) + 1;
const GLIDE_POLL_MS: u64 = 3;
const GLIDE_MAX_SAMPLES: usize = 8;

#[derive(Clone, Debug)]
struct Glide {
    samples: [(Instant, Offset); GLIDE_MAX_SAMPLES],
    last: u32,
    pressed: bool,
    t_step: Instant,
    vel: Vec2,
    rest: Vec2,
}

impl Default for Glide {
    #[inline]
    fn default() -> Self {
        let now = Instant::now();

        Glide {
            samples: [(now, Offset::ZERO); GLIDE_MAX_SAMPLES],
            last: 0,
            pressed: false,
            t_step: now,
            vel: Vec2::ZERO,
            rest: Vec2::ZERO,
        }
    }
}

impl Glide {
    fn press_start(&mut self) {
        let next = (self.last as usize + 1) % GLIDE_MAX_SAMPLES;
        self.samples[next] = (Instant::now(), Offset::ZERO);
        self.last = next.cast();
        self.pressed = true;
    }

    /// Returns true if component should immediately scroll by delta
    fn press_move(&mut self, delta: Offset) -> bool {
        let next = (self.last as usize + 1) % GLIDE_MAX_SAMPLES;
        self.samples[next] = (Instant::now(), delta);
        self.last = next.cast();
        self.vel == Vec2::ZERO
    }

    /// Returns true if momentum scrolling starts
    fn press_end(&mut self, timeout: Duration, pan_dist_thresh: f32) -> bool {
        self.pressed = false;

        let now = Instant::now();
        let mut delta = Offset::ZERO;
        let mut t0 = now;
        for (time, d) in &self.samples {
            if *time + timeout >= now {
                t0 = t0.min(*time);
                delta += *d;
            }
        }
        let dur = timeout; //now - t0;
        let mut is_start = false;
        if f32::conv(delta.distance_l_inf()) >= pan_dist_thresh {
            if self.vel == Vec2::ZERO {
                self.t_step = Instant::now();
                is_start = true;
            }
            self.vel += Vec2::conv(delta) / dur.as_secs_f32();
        } else {
            self.vel = Vec2::ZERO;
            self.rest = Vec2::ZERO;
        }
        is_start
    }

    fn step(&mut self, timeout: Duration, (decay_mul, decay_sub): (f32, f32)) -> Option<Offset> {
        // Stop on click+hold as well as min velocity. Do not stop on reaching
        // the maximum scroll offset since we might still be scrolling a parent!
        let stop = self.pressed && self.samples[self.last as usize].0.elapsed() > timeout;
        if stop || self.vel.abs().max_comp() < 1.0 {
            self.vel = Vec2::ZERO;
            self.rest = Vec2::ZERO;
            return None;
        }

        let now = Instant::now();
        let dur = (now - self.t_step).as_secs_f32();
        self.t_step = now;

        let v = self.vel * decay_mul.powf(dur);
        self.vel = v - v.abs().min(Vec2::splat(decay_sub * dur)) * v.sign();

        let d = self.vel * dur + self.rest;
        let delta = Offset::conv_trunc(d);
        self.rest = d - Vec2::conv(delta);

        Some(delta)
    }

    fn stop(&mut self) {
        self.vel = Vec2::ZERO;
        self.rest = Vec2::ZERO;
    }
}

/// Logic for a scroll region
///
/// This struct handles some scroll logic. It does not provide scroll bars.
#[derive(Clone, Debug, Default)]
pub struct ScrollComponent {
    max_offset: Offset,
    offset: Offset,
    glide: Glide,
}

impl ScrollComponent {
    /// True if momentum scrolling is active
    #[inline]
    pub fn is_gliding(&self) -> bool {
        self.glide.vel != Vec2::ZERO
    }

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
    /// Like [`Self::set_offset`] this generates a [`Action`] due to potential
    /// change in offset. In practice the caller will likely be performing all
    /// required updates regardless and the return value can be safely ignored.
    pub fn set_sizes(&mut self, window_size: Size, content_size: Size) -> Action {
        self.max_offset =
            (Offset::conv(content_size) - Offset::conv(window_size)).max(Offset::ZERO);
        self.set_offset(self.offset)
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`Action::empty()`] if the offset is identical to the old offset,
    /// or [`Action::REGION_MOVED`] if the offset changes.
    ///
    /// Also cancels any momentum scrolling, but only if `offset` is not equal
    /// to the current offset.
    pub fn set_offset(&mut self, offset: Offset) -> Action {
        let offset = offset.clamp(Offset::ZERO, self.max_offset);
        if offset == self.offset {
            Action::empty()
        } else {
            self.glide.stop();
            self.offset = offset;
            Action::REGION_MOVED
        }
    }

    /// Scroll to make the given `rect` visible
    ///
    /// Inputs:
    ///
    /// -   `rect`: the rect to focus in child's coordinate space
    /// -   `window_rect`: the rect of the scroll window
    ///
    /// Sets [`Scroll::Rect`] to ensure correct scrolling of parents.
    ///
    /// Returns `true` when the scroll offset changes.
    pub fn focus_rect(&mut self, cx: &mut EventCx, rect: Rect, window_rect: Rect) -> bool {
        self.glide.stop();
        let v = rect.pos - window_rect.pos;
        let off = Offset::conv(rect.size) - Offset::conv(window_rect.size);
        let offset = self.offset.max(v + off).min(v);
        let action = self.set_offset(offset);
        cx.set_scroll(Scroll::Rect(rect - self.offset));
        if action.is_empty() {
            false
        } else {
            *cx |= action;
            true
        }
    }

    /// Handle a [`Scroll`] action
    pub fn scroll(&mut self, cx: &mut EventCx, window_rect: Rect, scroll: Scroll) {
        match scroll {
            Scroll::None | Scroll::Scrolled => (),
            Scroll::Offset(delta) => {
                let old_offset = self.offset;
                *cx |= self.set_offset(old_offset - delta);
                cx.set_scroll(match delta - old_offset + self.offset {
                    delta if delta == Offset::ZERO => Scroll::Scrolled,
                    delta => Scroll::Offset(delta),
                });
            }
            Scroll::Rect(rect) => {
                self.focus_rect(cx, rect, window_rect);
            }
        }
    }

    fn scroll_by_delta(&mut self, cx: &mut EventCx, d: Offset) -> bool {
        let mut delta = d;
        let mut moved = false;
        let offset = (self.offset - d).clamp(Offset::ZERO, self.max_offset);
        if offset != self.offset {
            moved = true;
            delta = d - (self.offset - offset);
            self.offset = offset;
            *cx |= Action::REGION_MOVED;
        }

        cx.set_scroll(if delta != Offset::ZERO {
            Scroll::Offset(delta)
        } else {
            Scroll::Scrolled
        });

        moved
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
    /// Returns `(moved, response)` where `moved` means *this component
    /// scrolled* (scrolling of a parent is possible even if `!moved`).
    pub fn scroll_by_event(
        &mut self,
        cx: &mut EventCx,
        event: Event,
        id: WidgetId,
        window_rect: Rect,
    ) -> (bool, Response) {
        let mut moved = false;
        match event {
            Event::Command(cmd, _) => {
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
                            LineDelta(x, y) => cx.config().scroll_distance((x, y)),
                            PixelDelta(d) => d,
                        };
                        self.offset - delta
                    }
                };
                let action = self.set_offset(offset);
                if !action.is_empty() {
                    moved = true;
                    *cx |= action;
                }
                cx.set_scroll(Scroll::Rect(window_rect));
            }
            Event::Scroll(delta) => {
                let delta = match delta {
                    LineDelta(x, y) => cx.config().scroll_distance((x, y)),
                    PixelDelta(d) => d,
                };
                self.glide.stop();
                moved = self.scroll_by_delta(cx, delta);
            }
            Event::PressStart { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                let _ = press.grab(id).with_icon(CursorIcon::Grabbing).with_cx(cx);
                self.glide.press_start();
            }
            Event::PressMove { press, delta, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                if self.glide.press_move(delta) {
                    moved = self.scroll_by_delta(cx, delta);
                }
            }
            Event::PressEnd { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                let timeout = cx.config().scroll_flick_timeout();
                let pan_dist_thresh = cx.config().pan_dist_thresh();
                if self.glide.press_end(timeout, pan_dist_thresh) {
                    cx.request_timer_update(id, PAYLOAD_GLIDE, Duration::new(0, 0), true);
                }
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let timeout = cx.config().scroll_flick_timeout();
                let decay = cx.config().scroll_flick_decay();
                if let Some(delta) = self.glide.step(timeout, decay) {
                    moved = self.scroll_by_delta(cx, delta);

                    if self.glide.vel != Vec2::ZERO {
                        let dur = Duration::from_millis(GLIDE_POLL_MS);
                        cx.request_timer_update(id, PAYLOAD_GLIDE, dur, true);
                        cx.set_scroll(Scroll::Scrolled);
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
    pub fn handle(&mut self, cx: &mut EventCx, w_id: WidgetId, event: Event) -> TextInputAction {
        use TextInputAction as Action;
        match event {
            Event::PressStart { press } if press.is_primary() => {
                let (action, icon) = match *press {
                    PressSource::Touch(touch_id) => {
                        self.touch_phase = TouchPhase::Start(touch_id, press.coord);
                        let delay = cx.config().touch_select_delay();
                        cx.request_timer_update(w_id.clone(), PAYLOAD_SELECT, delay, false);
                        (Action::Focus, None)
                    }
                    PressSource::Mouse(..) if cx.config_enable_mouse_text_pan() => {
                        (Action::Focus, Some(CursorIcon::Grabbing))
                    }
                    PressSource::Mouse(_, repeats) => (
                        Action::Cursor(press.coord, true, !cx.modifiers().shift_key(), repeats),
                        None,
                    ),
                };
                press.grab(w_id).with_opt_icon(icon).with_cx(cx);
                self.glide.press_start();
                action
            }
            Event::PressMove { press, delta } if press.is_primary() => {
                self.glide.press_move(delta);
                match press.source {
                    PressSource::Touch(touch_id) => match self.touch_phase {
                        TouchPhase::Start(id, start_coord) if id == touch_id => {
                            let delta = press.coord - start_coord;
                            if cx.config_test_pan_thresh(delta) {
                                self.touch_phase = TouchPhase::Pan(id);
                                Action::Pan(delta)
                            } else {
                                Action::None
                            }
                        }
                        TouchPhase::Pan(id) if id == touch_id => Action::Pan(delta),
                        _ => Action::Cursor(press.coord, false, false, 1),
                    },
                    PressSource::Mouse(..) if cx.config_enable_mouse_text_pan() => {
                        Action::Pan(delta)
                    }
                    PressSource::Mouse(_, repeats) => {
                        Action::Cursor(press.coord, false, false, repeats)
                    }
                }
            }
            Event::PressEnd { press, .. } if press.is_primary() => {
                let timeout = cx.config().scroll_flick_timeout();
                let pan_dist_thresh = cx.config().pan_dist_thresh();
                if self.glide.press_end(timeout, pan_dist_thresh)
                    && (matches!(press.source, PressSource::Touch(id) if self.touch_phase == TouchPhase::Pan(id))
                        || matches!(press.source, PressSource::Mouse(..) if cx.config_enable_mouse_text_pan()))
                {
                    self.touch_phase = TouchPhase::None;
                    cx.request_timer_update(w_id, PAYLOAD_GLIDE, Duration::new(0, 0), true);
                }
                Action::None
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_SELECT => {
                match self.touch_phase {
                    TouchPhase::Start(touch_id, coord) => {
                        self.touch_phase = TouchPhase::Cursor(touch_id);
                        Action::Cursor(coord, true, !cx.modifiers().shift_key(), 1)
                    }
                    // Note: if the TimerUpdate were from another requester it
                    // should technically be Unused, but it doesn't matter
                    // so long as other consumers match this first.
                    _ => Action::None,
                }
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let timeout = cx.config().scroll_flick_timeout();
                let decay = cx.config().scroll_flick_decay();
                if let Some(delta) = self.glide.step(timeout, decay) {
                    let dur = Duration::from_millis(GLIDE_POLL_MS);
                    cx.request_timer_update(w_id, PAYLOAD_GLIDE, dur, true);
                    Action::Pan(delta)
                } else {
                    Action::None
                }
            }
            _ => Action::Unused,
        }
    }
}
