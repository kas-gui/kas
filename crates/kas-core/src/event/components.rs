// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::ScrollDelta::{LineDelta, PixelDelta};
use super::*;
use crate::cast::traits::*;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
#[allow(unused)]
use crate::text::{SelectionAction, SelectionHelper};
use crate::{Action, Id};
use kas_macros::impl_default;
use std::time::{Duration, Instant};

const TIMER_SELECT: u64 = 1 << 60;
const TIMER_GLIDE: u64 = (1 << 60) + 1;
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
    /// Returns [`Action::REGION_MOVED`] when the scroll offset changes.
    pub fn focus_rect(&mut self, cx: &mut EventCx, rect: Rect, window_rect: Rect) -> Action {
        let action = self.self_focus_rect(rect, window_rect);
        cx.set_scroll(Scroll::Rect(rect - self.offset));
        action
    }

    /// Scroll self to make the given `rect` visible
    ///
    /// This is identical to [`Self::focus_rect`] except that it does not call
    /// [`EventCx::set_scroll`], thus will not affect ancestors.
    #[cfg_attr(not(feature = "internal_doc"), doc(hidden))]
    #[cfg_attr(doc_cfg, doc(cfg(internal_doc)))]
    pub fn self_focus_rect(&mut self, rect: Rect, window_rect: Rect) -> Action {
        self.glide.stop();
        let v = rect.pos - window_rect.pos;
        let off = Offset::conv(rect.size) - Offset::conv(window_rect.size);
        let offset = self.offset.max(v + off).min(v);
        self.set_offset(offset)
    }

    /// Handle a [`Scroll`] action
    pub fn scroll(&mut self, cx: &mut EventCx, window_rect: Rect, scroll: Scroll) -> Action {
        match scroll {
            Scroll::None | Scroll::Scrolled => Action::empty(),
            Scroll::Offset(delta) => {
                let old_offset = self.offset;
                let action = self.set_offset(old_offset - delta);
                cx.set_scroll(match delta - old_offset + self.offset {
                    delta if delta == Offset::ZERO => Scroll::Scrolled,
                    delta => Scroll::Offset(delta),
                });
                action
            }
            Scroll::Rect(rect) => self.focus_rect(cx, rect, window_rect),
        }
    }

    // Returns Action::REGION_MOVED or Action::empty()
    fn scroll_by_delta(&mut self, cx: &mut EventCx, d: Offset) -> Action {
        let mut delta = d;
        let action;
        let offset = (self.offset - d).clamp(Offset::ZERO, self.max_offset);
        if offset != self.offset {
            delta = d - (self.offset - offset);
            self.offset = offset;
            action = Action::REGION_MOVED;
        } else {
            action = Action::empty();
        }

        cx.set_scroll(if delta != Offset::ZERO {
            Scroll::Offset(delta)
        } else {
            Scroll::Scrolled
        });

        action
    }

    /// Use an event to scroll, if possible
    ///
    /// Consumes the following events: `Command`, `Scroll`, `PressStart`,
    /// `PressMove`, `PressEnd`, `Timer(pl)` where `pl == (1<<60) + 1`.
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
    /// Returns `(moved, is_used)` where `moved` means *this component
    /// scrolled* (scrolling of a parent is possible even if `!moved`).
    pub fn scroll_by_event(
        &mut self,
        cx: &mut EventCx,
        event: Event,
        id: Id,
        window_rect: Rect,
    ) -> (bool, IsUsed) {
        let mut action = Action::empty();
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
                            _ => return (false, Unused),
                        };
                        let delta = match delta {
                            LineDelta(x, y) => cx.config().event().scroll_distance((x, y)),
                            PixelDelta(d) => d,
                        };
                        self.offset - delta
                    }
                };
                action = self.set_offset(offset);
                cx.set_scroll(Scroll::Rect(window_rect));
            }
            Event::Scroll(delta) => {
                let delta = match delta {
                    LineDelta(x, y) => cx.config().event().scroll_distance((x, y)),
                    PixelDelta(d) => d,
                };
                self.glide.stop();
                action = self.scroll_by_delta(cx, delta);
            }
            Event::PressStart { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                let _ = press
                    .grab(id.clone())
                    .with_icon(CursorIcon::Grabbing)
                    .with_cx(cx);
                self.glide.press_start();
            }
            Event::PressMove { press, delta, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                if self.glide.press_move(delta) {
                    action = self.scroll_by_delta(cx, delta);
                }
            }
            Event::PressEnd { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                let timeout = cx.config().event().scroll_flick_timeout();
                let pan_dist_thresh = cx.config().event().pan_dist_thresh();
                if self.glide.press_end(timeout, pan_dist_thresh) {
                    cx.request_timer(id.clone(), TIMER_GLIDE, Duration::new(0, 0));
                }
            }
            Event::Timer(pl) if pl == TIMER_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let timeout = cx.config().event().scroll_flick_timeout();
                let decay = cx.config().event().scroll_flick_decay();
                if let Some(delta) = self.glide.step(timeout, decay) {
                    action = self.scroll_by_delta(cx, delta);

                    if self.glide.vel != Vec2::ZERO {
                        let dur = Duration::from_millis(GLIDE_POLL_MS);
                        cx.request_timer(id.clone(), TIMER_GLIDE, dur);
                        cx.set_scroll(Scroll::Scrolled);
                    }
                }
            }
            _ => return (false, Unused),
        }
        if !action.is_empty() {
            cx.action(id, action);
            (true, Used)
        } else {
            (false, Used)
        }
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
    /// Focus, optionally updating position and selection
    ///
    /// To handle:
    ///
    /// 1.  If a `coord` is included, translate to a text index then call
    ///     [`SelectionHelper::set_edit_pos`].
    /// 2.  Call [`SelectionHelper::action`].
    /// 3.  If supporting the primary buffer (Unix), set its contents now if the
    ///     widget has selection focus or otherwise when handling
    ///     [`Event::SelFocus`] for a pointer source.
    /// 4.  Request keyboard or selection focus if not already gained.
    Focus {
        coord: Option<Coord>,
        action: SelectionAction,
    },
}

impl TextInput {
    /// Handle input events
    ///
    /// Consumes the following events: `PressStart`, `PressMove`, `PressEnd`,
    /// `Timer(pl)` where `pl == 1<<60 || pl == (1<<60)+1`.
    /// May request press grabs and timer updates.
    ///
    /// Implements scrolling and text selection behaviour, excluding handling of
    /// [`Event::Scroll`].
    pub fn handle(&mut self, cx: &mut EventCx, w_id: Id, event: Event) -> TextInputAction {
        use TextInputAction as Action;
        match event {
            Event::PressStart { press } if press.is_primary() => {
                let mut action = Action::Focus {
                    coord: None,
                    action: SelectionAction::default(),
                };
                let icon = match *press {
                    PressSource::Touch(touch_id) => {
                        self.touch_phase = TouchPhase::Start(touch_id, press.coord);
                        let delay = cx.config().event().touch_select_delay();
                        cx.request_timer(w_id.clone(), TIMER_SELECT, delay);
                        None
                    }
                    PressSource::Mouse(..) if cx.config_enable_mouse_text_pan() => {
                        Some(CursorIcon::Grabbing)
                    }
                    PressSource::Mouse(_, repeats) => {
                        action = Action::Focus {
                            coord: Some(press.coord),
                            action: SelectionAction {
                                anchor: true,
                                clear: !cx.modifiers().shift_key(),
                                repeats,
                            },
                        };
                        None
                    }
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
                        _ => Action::Focus {
                            coord: Some(press.coord),
                            action: SelectionAction::new(false, false, 1),
                        },
                    },
                    PressSource::Mouse(..) if cx.config_enable_mouse_text_pan() => {
                        Action::Pan(delta)
                    }
                    PressSource::Mouse(_, repeats) => Action::Focus {
                        coord: Some(press.coord),
                        action: SelectionAction::new(false, false, repeats),
                    },
                }
            }
            Event::PressEnd { press, .. } if press.is_primary() => {
                let timeout = cx.config().event().scroll_flick_timeout();
                let pan_dist_thresh = cx.config().event().pan_dist_thresh();
                if self.glide.press_end(timeout, pan_dist_thresh)
                    && (matches!(press.source, PressSource::Touch(id) if self.touch_phase == TouchPhase::Pan(id))
                        || matches!(press.source, PressSource::Mouse(..) if cx.config_enable_mouse_text_pan()))
                {
                    self.touch_phase = TouchPhase::None;
                    cx.request_timer(w_id, TIMER_GLIDE, Duration::new(0, 0));
                }
                Action::None
            }
            Event::Timer(pl) if pl == TIMER_SELECT => match self.touch_phase {
                TouchPhase::Start(touch_id, coord) => {
                    self.touch_phase = TouchPhase::Cursor(touch_id);
                    Action::Focus {
                        coord: Some(coord),
                        action: SelectionAction::new(true, !cx.modifiers().shift_key(), 1),
                    }
                }
                _ => Action::None,
            },
            Event::Timer(pl) if pl == TIMER_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let timeout = cx.config().event().scroll_flick_timeout();
                let decay = cx.config().event().scroll_flick_decay();
                if let Some(delta) = self.glide.step(timeout, decay) {
                    let dur = Duration::from_millis(GLIDE_POLL_MS);
                    cx.request_timer(w_id, TIMER_GLIDE, dur);
                    Action::Pan(delta)
                } else {
                    Action::None
                }
            }
            _ => Action::Unused,
        }
    }
}
