// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::ScrollDelta::{LineDelta, PixelDelta};
use super::{Command, Event, EventMgr, GrabMode, PressSource, Response, VoidMsg};
use crate::cast::CastFloat;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
#[allow(unused)]
use crate::text::SelectionHelper;
use crate::{TkAction, WidgetId};
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
            let v = Vec2::from(delta) / dur.as_secs_f32();
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
            let rest = d.fract();
            let delta = Offset::from(d.trunc());

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
    scroll_rate: f32,
    glide: Glide,
}

impl Default for ScrollComponent {
    #[inline]
    fn default() -> Self {
        ScrollComponent {
            max_offset: Offset::ZERO,
            offset: Offset::ZERO,
            scroll_rate: 30.0,
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
        self.max_offset = Offset::from(content_size) - Offset::from(window_size);
        self.set_offset(self.offset)
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`TkAction::empty()`] if the offset is identical to the old offset,
    /// or [`TkAction::REGION_MOVED`] if the offset changes.
    #[inline]
    pub fn set_offset(&mut self, offset: Offset) -> TkAction {
        let offset = offset.clamp(Offset::ZERO, self.max_offset);
        if offset == self.offset {
            TkAction::empty()
        } else {
            self.offset = offset;
            TkAction::REGION_MOVED
        }
    }

    /// Set the scroll rate
    ///
    /// This affects how fast arrow keys and the mouse wheel scroll (but not
    /// pixel offsets, as from touch devices supporting smooth scrolling).
    #[inline]
    pub fn set_scroll_rate(&mut self, rate: f32) {
        self.scroll_rate = rate;
    }

    /// Apply offset to an event being sent to the scrolled child
    #[inline]
    pub fn offset_event(&self, mut event: Event) -> Event {
        match &mut event {
            Event::PressStart { coord, .. } => {
                *coord += self.offset;
            }
            Event::PressMove { coord, .. } => {
                *coord += self.offset;
            }
            Event::PressEnd { coord, .. } => {
                *coord += self.offset;
            }
            _ => {}
        };
        event
    }

    /// Handle [`Response::Focus`]
    ///
    /// Inputs and outputs:
    ///
    /// -   `rect`: the focus rect
    /// -   `window_rect`: the rect of the scroll window
    /// -   returned `Rect`: the focus rect, adjusted for scroll offset; normally this should be
    ///     returned via another [`Response::Focus`]
    /// -   returned `TkAction`: action to pass to the event manager
    #[inline]
    pub fn focus_rect(&mut self, rect: Rect, window_rect: Rect) -> (Rect, TkAction) {
        let v = rect.pos - window_rect.pos;
        let off = Offset::from(rect.size) - Offset::from(window_rect.size);
        let offset = self.offset.max(v + off).min(v);
        let action = self.set_offset(offset);
        (rect - self.offset, action)
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
    /// If the `on_press_start` closure requests a mouse grab, this also
    /// implements scrolling on `PressMove` mouse/touch events. On release
    /// (`PressEnd`), given sufficient speed, momentum scrolling commences.
    /// The `on_press_start` closure may choose to request a mouse grab only
    /// given certain conditions, e.g. only on the primary mouse button:
    /// ```
    /// # use kas::prelude::*;
    /// # type Msg = ();
    /// fn dummy_event_handler(
    ///     id: WidgetId,
    ///     scroll: &mut kas_widgets::ScrollComponent,
    ///     mgr: &mut EventMgr,
    ///     event: Event
    /// )
    ///     -> Response<Msg>
    /// {
    ///     let window_size = Size(100, 80);
    ///     let (action, response) = scroll.scroll_by_event(
    ///         mgr,
    ///         event,
    ///         id.clone(),
    ///         window_size,
    ///         |mgr, source, _, coord| if source.is_primary() {
    ///             let icon = Some(kas::event::CursorIcon::Grabbing);
    ///             mgr.request_grab(id, source, coord, kas::event::GrabMode::Grab, icon);
    ///         }
    ///     );
    ///     *mgr |= action;
    ///     response.void_into()
    /// }
    /// ```
    ///
    /// If the returned [`TkAction`] is `None`, the scroll offset has not changed and
    /// the returned [`Response`] is either `Used` or `Unused`.
    /// If the returned [`TkAction`] is not `None`, the scroll offset has been
    /// updated and the second return value is `Response::Used`.
    #[inline]
    pub fn scroll_by_event<PS: FnOnce(&mut EventMgr, PressSource, WidgetId, Coord)>(
        &mut self,
        mgr: &mut EventMgr,
        event: Event,
        id: WidgetId,
        window_size: Size,
        on_press_start: PS,
    ) -> (TkAction, Response<VoidMsg>) {
        let mut action = TkAction::empty();
        let mut response = Response::Used;

        match event {
            Event::Command(Command::Home, _) => {
                action = self.set_offset(Offset::ZERO);
            }
            Event::Command(Command::End, _) => {
                action = self.set_offset(self.max_offset);
            }
            Event::Command(cmd, _) => {
                let delta = match cmd {
                    Command::Left => LineDelta(-1.0, 0.0),
                    Command::Right => LineDelta(1.0, 0.0),
                    Command::Up => LineDelta(0.0, 1.0),
                    Command::Down => LineDelta(0.0, -1.0),
                    Command::PageUp => PixelDelta(Offset(0, window_size.1 / 2)),
                    Command::PageDown => PixelDelta(Offset(0, -(window_size.1 / 2))),
                    _ => return (action, Response::Unused),
                };

                let d = match delta {
                    LineDelta(x, y) => Offset(
                        (-self.scroll_rate * x).cast_nearest(),
                        (self.scroll_rate * y).cast_nearest(),
                    ),
                    PixelDelta(d) => d,
                };
                action = self.set_offset(self.offset - d);
            }
            Event::Scroll(delta) => {
                let d = match delta {
                    LineDelta(x, y) => Offset(
                        (-self.scroll_rate * x).cast_nearest(),
                        (self.scroll_rate * y).cast_nearest(),
                    ),
                    PixelDelta(d) => d,
                };
                let old_offset = self.offset;
                action = self.set_offset(old_offset - d);
                let delta = d - (old_offset - self.offset);
                response = if delta != Offset::ZERO {
                    Response::Pan(delta)
                } else {
                    Response::Scrolled
                };
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => on_press_start(mgr, source, start_id, coord),
            Event::PressMove { mut delta, .. } => {
                self.glide.move_delta(delta);
                let old_offset = self.offset;
                action = self.set_offset(old_offset - delta);
                delta = old_offset - self.offset;
                response = if delta != Offset::ZERO {
                    Response::Pan(delta)
                } else {
                    Response::Scrolled
                };
            }
            Event::PressEnd { .. } => {
                if self.glide.opt_start(mgr.config().scroll_flick_timeout()) {
                    mgr.update_on_timer(Duration::new(0, 0), id, PAYLOAD_GLIDE);
                }
            }
            Event::TimerUpdate(pl) if pl == PAYLOAD_GLIDE => {
                // Momentum/glide scrolling: update per arbitrary step time until movment stops.
                let decay = mgr.config().scroll_flick_decay();
                if let Some(delta) = self.glide.step(decay) {
                    action = self.set_offset(self.offset - delta);
                    mgr.update_on_timer(Duration::from_millis(GLIDE_POLL_MS), id, PAYLOAD_GLIDE);
                    response = Response::Scrolled;
                }
            }
            _ => response = Response::Unused,
        }
        (action, response)
    }
}

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
                let grab = mgr.request_grab(w_id.clone(), source, coord, GrabMode::Grab, None);
                match source {
                    PressSource::Touch(touch_id) => {
                        if grab && self.touch_phase == TouchPhase::None {
                            self.touch_phase = TouchPhase::Start(touch_id, coord);
                            let delay = mgr.config().touch_text_sel_delay();
                            mgr.update_on_timer(delay, w_id, PAYLOAD_SELECT);
                        }
                        Action::Focus
                    }
                    PressSource::Mouse(..) if mgr.config_enable_mouse_text_pan() => Action::Focus,
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
                self.glide.move_delta(delta);
                match source {
                    PressSource::Touch(touch_id) => match self.touch_phase {
                        TouchPhase::None => {
                            self.touch_phase = TouchPhase::Pan(touch_id);
                            Action::Pan(delta)
                        }
                        TouchPhase::Start(id, start_coord) if id == touch_id => {
                            if mgr.config_test_pan_thresh(coord - start_coord) {
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
                if self.glide.opt_start(mgr.config().scroll_flick_timeout()) {
                    if matches!(source, PressSource::Touch(id) if self.touch_phase == TouchPhase::Pan(id))
                        || matches!(source, PressSource::Mouse(..) if mgr.config_enable_mouse_text_pan())
                    {
                        mgr.update_on_timer(Duration::new(0, 0), w_id, PAYLOAD_GLIDE);
                    }
                }
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
            Event::TimerUpdate(pl) if pl == PAYLOAD_SELECT => {
                match self.touch_phase {
                    TouchPhase::Start(touch_id, coord) => {
                        self.touch_phase = TouchPhase::Cursor(touch_id);
                        Action::Cursor(coord, false, !mgr.modifiers().shift(), 1)
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
                    mgr.update_on_timer(Duration::from_millis(GLIDE_POLL_MS), w_id, 0);
                    Action::Pan(delta)
                } else {
                    Action::None
                }
            }
            _ => Action::Unused,
        }
    }
}
