// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::*;
use crate::cast::traits::*;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
#[allow(unused)]
use crate::text::{SelectionAction, SelectionHelper};
use crate::{Action, Id};
use kas_macros::impl_default;
use std::time::Instant;

const TIMER_SELECT: TimerHandle = TimerHandle::new(1 << 60, true);
const TIMER_KINETIC: TimerHandle = TimerHandle::new((1 << 60) + 1, true);
const KINETIC_RESIDUAL_VEL_REDUCTION_FACTOR: f32 = 0.5;

/// Details used to initiate kinetic scrolling
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KineticStart {
    vel: Vec2,
    rest: Vec2,
}

/// Kinetic scrolling model
#[derive(Clone, Debug)]
pub struct Kinetic {
    press: Option<PressSource>,
    t_step: Instant,
    vel: Vec2,
    rest: Vec2,
}

impl Default for Kinetic {
    #[inline]
    fn default() -> Self {
        let now = Instant::now();

        Kinetic {
            press: None,
            t_step: now,
            vel: Vec2::ZERO,
            rest: Vec2::ZERO,
        }
    }
}

impl Kinetic {
    /// Call on [`Event::PressStart`]
    pub fn press_start(&mut self, press: PressSource) {
        self.press = Some(press);
    }

    /// Call on [`Event::PressMove`]
    ///
    /// Returns true if component should immediately scroll by delta
    pub fn press_move(&mut self, press: PressSource) -> bool {
        self.press == Some(press) && self.vel == Vec2::ZERO
    }

    /// Call on [`Event::PressEnd`]
    ///
    /// Returns true when a frame timer ([`EventState::request_frame_timer`])
    /// should be requested (see [`Self::step`]).
    pub fn press_end(&mut self, press: PressSource, vel: Vec2) -> bool {
        if self.press != Some(press) {
            return false;
        }
        self.press = None;

        self.vel += vel;
        if self.vel.distance_l_inf() < 1.0 {
            self.stop();
            false
        } else {
            self.t_step = Instant::now();
            true
        }
    }

    /// Call on [`Scroll::Kinetic`] to immediately start (or accelerate) scrolling
    ///
    /// Returns any offset which should be applied immediately.
    pub fn start(&mut self, start: KineticStart) -> Offset {
        self.vel += start.vel;
        let d = self.rest + start.rest;
        let delta = Offset::conv_trunc(d);
        self.rest = d - Vec2::conv(delta);
        self.t_step = Instant::now();
        delta
    }

    /// Call this regularly using a frame timer
    ///
    /// On [`Self::press_end`] and while in motion ([`Self::is_scrolling`]), a
    /// frame timer ([`EventState::request_frame_timer`]) should be requested
    /// and used to call this method.
    pub fn step(&mut self, cx: &EventState) -> Option<Offset> {
        let evc = cx.config().event();
        let now = Instant::now();
        let dur = (now - self.t_step).as_secs_f32();
        self.t_step = now;

        if let Some(source) = self.press {
            let decay_sub = evc.kinetic_grab_sub();
            let grab_vel = cx.press_velocity(source).unwrap_or_default() + self.vel;

            let v = self.vel - grab_vel;
            self.vel -= v.abs().min(Vec2::splat(decay_sub * dur)) * v.sign();
        }

        let (decay_mul, decay_sub) = evc.kinetic_decay();
        let v = self.vel * decay_mul.powf(dur);
        self.vel = v - v.abs().min(Vec2::splat(decay_sub * dur)) * v.sign();

        if self.press.is_none() && self.vel.distance_l_inf() < 1.0 {
            self.stop();
            return None;
        }

        let d = self.vel * dur + self.rest;
        let delta = Offset::conv_trunc(d);
        self.rest = d - Vec2::conv(delta);

        Some(delta)
    }

    /// Stop scrolling immediately
    #[inline]
    pub fn stop(&mut self) {
        self.vel = Vec2::ZERO;
        self.rest = Vec2::ZERO;
    }

    /// Stop scrolling on any axis were `delta` is non-zero
    ///
    /// Returns a [`KineticStart`] message from the residual velocity and delta.
    pub fn stop_with_residual(&mut self, delta: Offset) -> KineticStart {
        let mut start = KineticStart::default();
        if delta.0 != 0 {
            start.vel.0 = self.vel.0 * KINETIC_RESIDUAL_VEL_REDUCTION_FACTOR;
            start.rest.0 = self.rest.0 + f32::conv(delta.0);
            self.vel.0 = 0.0;
            self.rest.0 = 0.0;
        }
        if delta.1 != 0 {
            start.vel.1 = self.vel.1 * KINETIC_RESIDUAL_VEL_REDUCTION_FACTOR;
            start.rest.1 = self.rest.1 + f32::conv(delta.1);
            self.vel.1 = 0.0;
            self.rest.1 = 0.0;
        }
        start
    }

    /// True while kinetic scrolling
    #[inline]
    pub fn is_scrolling(&self) -> bool {
        self.vel != Vec2::ZERO
    }
}

/// Logic for a scroll region
///
/// This struct handles some scroll logic. It does not provide scroll bars.
#[derive(Clone, Debug, Default)]
pub struct ScrollComponent {
    max_offset: Offset,
    offset: Offset,
    kinetic: Kinetic,
}

impl ScrollComponent {
    /// True if kinetic scrolling is active
    #[inline]
    pub fn is_kinetic_scrolling(&self) -> bool {
        self.kinetic.is_scrolling()
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
    /// Also cancels any kinetic scrolling, but only if `offset` is not equal
    /// to the current offset.
    pub fn set_offset(&mut self, offset: Offset) -> Action {
        let offset = offset.clamp(Offset::ZERO, self.max_offset);
        if offset == self.offset {
            Action::empty()
        } else {
            self.kinetic.stop();
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
    #[cfg_attr(docsrs, doc(cfg(internal_doc)))]
    pub fn self_focus_rect(&mut self, rect: Rect, window_rect: Rect) -> Action {
        self.kinetic.stop();
        let v = rect.pos - window_rect.pos;
        let off = Offset::conv(rect.size) - Offset::conv(window_rect.size);
        let offset = self.offset.max(v + off).min(v);
        self.set_offset(offset)
    }

    /// Handle a [`Scroll`] action
    pub fn scroll(&mut self, cx: &mut EventCx, id: Id, window_rect: Rect, scroll: Scroll) {
        match scroll {
            Scroll::None | Scroll::Scrolled => (),
            Scroll::Offset(delta) => {
                let old_offset = self.offset;
                let action = self.set_offset(old_offset - delta);
                cx.action(id, action);
                cx.set_scroll(match delta - old_offset + self.offset {
                    delta if delta == Offset::ZERO => Scroll::Scrolled,
                    delta => Scroll::Offset(delta),
                });
            }
            Scroll::Kinetic(start) => {
                let delta = self.kinetic.start(start);
                let delta = self.scroll_self_by_delta(cx, id.clone(), delta);
                if delta == Offset::ZERO {
                    if self.kinetic.is_scrolling() {
                        cx.request_frame_timer(id, TIMER_KINETIC);
                    }
                    cx.set_scroll(Scroll::Scrolled);
                } else {
                    cx.set_scroll(Scroll::Kinetic(self.kinetic.stop_with_residual(delta)));
                }
            }
            Scroll::Rect(rect) => {
                let action = self.focus_rect(cx, rect, window_rect);
                cx.action(id, action);
            }
        }
    }

    /// Scroll self, returning any excess delta
    ///
    /// Caller is expected to call [`EventCx::set_scroll`].
    fn scroll_self_by_delta(&mut self, cx: &mut EventCx, id: Id, d: Offset) -> Offset {
        let mut delta = d;
        let offset = (self.offset - d).clamp(Offset::ZERO, self.max_offset);
        if offset != self.offset {
            delta = d - (self.offset - offset);
            self.offset = offset;
            cx.action(id, Action::REGION_MOVED);
        }
        delta
    }

    fn scroll_by_delta(&mut self, cx: &mut EventCx, id: Id, d: Offset) {
        let delta = self.scroll_self_by_delta(cx, id, d);
        cx.set_scroll(if delta != Offset::ZERO {
            Scroll::Offset(delta)
        } else {
            Scroll::Scrolled
        });
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
    /// `PressEnd` initiates kinetic-scrolling if the speed is high enough.
    pub fn scroll_by_event(
        &mut self,
        cx: &mut EventCx,
        event: Event,
        id: Id,
        window_rect: Rect,
    ) -> IsUsed {
        match event {
            Event::Command(cmd, _) => {
                let offset = match cmd {
                    Command::Home => Offset::ZERO,
                    Command::End => self.max_offset,
                    cmd => {
                        let delta = match cmd {
                            Command::Left => ScrollDelta::Lines(-1.0, 0.0),
                            Command::Right => ScrollDelta::Lines(1.0, 0.0),
                            Command::Up => ScrollDelta::Lines(0.0, 1.0),
                            Command::Down => ScrollDelta::Lines(0.0, -1.0),
                            Command::PageUp => {
                                ScrollDelta::Pixels(Offset(0, window_rect.size.1 / 2))
                            }
                            Command::PageDown => {
                                ScrollDelta::Pixels(Offset(0, -(window_rect.size.1 / 2)))
                            }
                            _ => return Unused,
                        };
                        self.offset - delta.as_offset(cx)
                    }
                };
                cx.action(id, self.set_offset(offset));
                cx.set_scroll(Scroll::Rect(window_rect));
            }
            Event::Scroll(delta) => {
                self.kinetic.stop();
                self.scroll_by_delta(cx, id, delta.as_offset(cx));
            }
            Event::PressStart { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                let _ = press
                    .grab(id, GrabMode::Grab)
                    .with_icon(CursorIcon::Grabbing)
                    .complete(cx);
                self.kinetic.press_start(press.source);
            }
            Event::PressMove { press, delta }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                if self.kinetic.press_move(press.source) {
                    self.scroll_by_delta(cx, id, delta);
                }
            }
            Event::PressEnd { press, .. }
                if self.max_offset != Offset::ZERO && cx.config_enable_pan(*press) =>
            {
                if let Some(velocity) = cx.press_velocity(press.source)
                    && self.kinetic.press_end(press.source, velocity)
                {
                    cx.request_frame_timer(id, TIMER_KINETIC);
                }
            }
            Event::Timer(TIMER_KINETIC) => {
                if let Some(delta) = self.kinetic.step(cx) {
                    let delta = self.scroll_self_by_delta(cx, id.clone(), delta);
                    let scroll = if delta == Offset::ZERO {
                        Scroll::Scrolled
                    } else {
                        Scroll::Kinetic(self.kinetic.stop_with_residual(delta))
                    };
                    cx.set_scroll(scroll);
                }

                if self.kinetic.is_scrolling() {
                    cx.request_frame_timer(id, TIMER_KINETIC);
                }
            }
            _ => return Unused,
        }
        Used
    }
}

#[impl_default(Phase::None)]
#[derive(Clone, Debug, PartialEq)]
enum Phase {
    None,
    Start(PressSource, Coord), // source, coord
    Pan(PressSource),          // source
    Cursor(PressSource),       // source
}

/// Handles text selection and panning from mouse and touch events
#[derive(Clone, Debug, Default)]
pub struct TextInput {
    phase: Phase,
}

/// Result of [`TextInput::handle`]
pub enum TextInputAction {
    /// Event is used, no action
    Used,
    /// Event not used
    Unused,
    /// Set the text cursor near to `coord` (mouse or touch position)
    ///
    /// If `action.anchor`, this is a new set-focus action; the selection anchor
    /// should be set to the current position (this is used to expand the
    /// selection on double- or triple-click). If `!action.anchor`, this is an
    /// update due to pointer motion used to drag a selection.
    ///
    /// To handle:
    ///
    /// 1.  Translate `coord` to a text index and call [`SelectionHelper::set_edit_pos`].
    /// 2.  Call [`SelectionHelper::action`].
    /// 3.  If supporting the primary buffer (Unix), set its contents now if the
    ///     widget has selection focus or otherwise when handling
    ///     [`Event::SelFocus`] for a pointer source.
    /// 4.  Request keyboard or selection focus if not already gained.
    Focus {
        coord: Coord,
        action: SelectionAction,
    },
    /// Current action is concluded
    Finish,
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
                let mut action = Action::Used;
                let icon = match *press {
                    PressSource::Touch(_) => {
                        self.phase = Phase::Start(*press, press.coord);
                        let delay = cx.config().event().touch_select_delay();
                        cx.request_timer(w_id.clone(), TIMER_SELECT, delay);
                        None
                    }
                    PressSource::Mouse(..) if cx.config_enable_mouse_text_pan() => {
                        self.phase = Phase::Pan(*press);
                        Some(CursorIcon::Grabbing)
                    }
                    PressSource::Mouse(_, repeats) => {
                        self.phase = Phase::Cursor(*press);
                        action = Action::Focus {
                            coord: press.coord,
                            action: SelectionAction {
                                anchor: true,
                                clear: !cx.modifiers().shift_key(),
                                repeats,
                            },
                        };
                        None
                    }
                };
                press
                    .grab(w_id, GrabMode::Grab)
                    .with_opt_icon(icon)
                    .complete(cx);
                action
            }
            Event::PressMove { press, delta } => match self.phase {
                Phase::Start(source, start_coord) if *press == source => {
                    let delta = press.coord - start_coord;
                    if cx.config_test_pan_thresh(delta) {
                        self.phase = Phase::Pan(source);
                        cx.set_scroll(Scroll::Offset(delta));
                    }
                    Action::Used
                }
                Phase::Pan(source) if *press == source => {
                    cx.set_scroll(Scroll::Offset(delta));
                    Action::Used
                }
                Phase::Cursor(source) if *press == source => {
                    let repeats = match *press {
                        PressSource::Touch(_) => 1,
                        PressSource::Mouse(_, n) => n,
                    };
                    Action::Focus {
                        coord: press.coord,
                        action: SelectionAction::new(false, false, repeats),
                    }
                }
                _ => Action::Used,
            },
            Event::PressEnd { press, .. } => {
                if let Phase::Pan(source) = self.phase
                    && *press == source
                    && let Some(vel) = cx.press_velocity(source)
                {
                    let rest = Vec2::ZERO;
                    cx.set_scroll(Scroll::Kinetic(KineticStart { vel, rest }));
                }
                self.phase = Phase::None;
                Action::Finish
            }
            Event::Timer(TIMER_SELECT) => match self.phase {
                Phase::Start(touch_id, coord) => {
                    self.phase = Phase::Cursor(touch_id);
                    Action::Focus {
                        coord,
                        action: SelectionAction::new(true, !cx.modifiers().shift_key(), 1),
                    }
                }
                _ => Action::Unused,
            },
            _ => Action::Unused,
        }
    }
}
