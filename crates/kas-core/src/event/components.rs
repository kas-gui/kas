// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling components

use super::*;
use crate::cast::traits::*;
use crate::geom::{Coord, Offset, Rect, Size, Vec2};
use crate::{ActionMoved, Id};
use kas_macros::{autoimpl, impl_default};
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
            let v = cx.press_velocity(source).unwrap_or_default();
            self.vel -= v.abs().min(Vec2::splat(decay_sub * dur)) * -v.sign();
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
    /// Returns an [`ActionMoved`] indicating whether the scroll offset changed.
    pub fn set_sizes(&mut self, window_size: Size, content_size: Size) -> ActionMoved {
        let max_offset = (Offset::conv(content_size) - Offset::conv(window_size)).max(Offset::ZERO);
        if max_offset == self.max_offset {
            return ActionMoved(false);
        }
        self.max_offset = max_offset;
        self.set_offset(self.offset)
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    ///
    /// Also cancels any kinetic scrolling, but only if `offset` is not equal
    /// to the current offset.
    pub fn set_offset(&mut self, offset: Offset) -> ActionMoved {
        let offset = offset.clamp(Offset::ZERO, self.max_offset);
        if offset == self.offset {
            ActionMoved(false)
        } else {
            self.kinetic.stop();
            self.offset = offset;
            ActionMoved(true)
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
    pub fn focus_rect(&mut self, cx: &mut EventCx, rect: Rect, window_rect: Rect) -> ActionMoved {
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
    pub fn self_focus_rect(&mut self, rect: Rect, window_rect: Rect) -> ActionMoved {
        self.kinetic.stop();
        let max_vis = rect.pos - window_rect.pos;
        let extra_size = Offset::conv(rect.size) - Offset::conv(window_rect.size);
        let min_vis = max_vis + extra_size;
        let center = max_vis + extra_size / 2;
        let lb = (min_vis + center) / 2;
        let ub = (max_vis + center) / 2;
        let offset = self.offset.max(lb).min(ub);
        self.set_offset(offset)
    }

    /// Handle a [`Scroll`] action
    pub fn scroll(&mut self, cx: &mut EventCx, id: Id, window_rect: Rect, scroll: Scroll) {
        match scroll {
            Scroll::None | Scroll::Scrolled => (),
            Scroll::Offset(delta) => {
                self.scroll_by_delta(cx, delta);
            }
            Scroll::Kinetic(start) => {
                let delta = self.kinetic.start(start);
                let delta = self.scroll_self_by_delta(cx, delta);
                if delta == Offset::ZERO {
                    cx.set_scroll(Scroll::Scrolled);
                } else {
                    cx.set_scroll(Scroll::Kinetic(self.kinetic.stop_with_residual(delta)));
                }
                if self.kinetic.is_scrolling() {
                    cx.request_frame_timer(id, TIMER_KINETIC);
                }
            }
            Scroll::Rect(rect) => {
                let action = self.focus_rect(cx, rect, window_rect);
                cx.action_moved(action);
            }
        }
    }

    /// Scroll self, returning any excess delta
    ///
    /// Caller is expected to call [`EventCx::set_scroll`].
    fn scroll_self_by_delta(&mut self, cx: &mut EventState, d: Offset) -> Offset {
        let mut delta = d;
        let offset = (self.offset - d).clamp(Offset::ZERO, self.max_offset);
        if offset != self.offset {
            delta = d - (self.offset - offset);
            self.offset = offset;
            cx.region_moved();
        }
        delta
    }

    fn scroll_by_delta(&mut self, cx: &mut EventCx, d: Vec2) {
        let delta = d + self.kinetic.rest;
        let offset = delta.cast_nearest();
        self.kinetic.rest = delta - Vec2::conv(offset);
        let delta = self.scroll_self_by_delta(cx, offset);
        cx.set_scroll(if delta != Offset::ZERO {
            Scroll::Offset(delta.cast())
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
                            Command::Left => ScrollDelta::Lines(1.0, 0.0),
                            Command::Right => ScrollDelta::Lines(-1.0, 0.0),
                            Command::Up => ScrollDelta::Lines(0.0, 1.0),
                            Command::Down => ScrollDelta::Lines(0.0, -1.0),
                            Command::PageUp | Command::PageDown => {
                                let mut v = 0.5 * f32::conv(window_rect.size.1);
                                if cmd == Command::PageDown {
                                    v = -v;
                                }
                                ScrollDelta::PixelDelta(Vec2(0.0, v))
                            }
                            _ => return Unused,
                        };
                        self.scroll_by_delta(cx, delta.as_offset(cx));
                        return Used;
                    }
                };
                cx.action_moved(self.set_offset(offset));
                cx.set_scroll(Scroll::Rect(window_rect));
            }
            Event::Scroll(delta) => {
                self.kinetic.stop();
                self.scroll_by_delta(cx, delta.as_offset(cx));
            }
            Event::PressStart(press)
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
                    self.scroll_by_delta(cx, delta);
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
                    let delta = self.scroll_self_by_delta(cx, delta);
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

#[impl_default(TextPhase::None)]
#[autoimpl(Clone, Debug, PartialEq)]
enum TextPhase {
    None,
    PressStart(PressSource, Coord), // source, coord
    Pan(PressSource),               // source
    Cursor(PressSource, Coord),     // source
}

/// Handles text selection and panning from mouse and touch events
#[derive(Clone, Debug, Default)]
pub struct TextInput {
    phase: TextPhase,
}

/// Result of [`TextInput::handle`]
#[autoimpl(Clone, Debug)]
pub enum TextInputAction {
    /// Event is used, no action
    Used,
    /// Event not used
    Unused,
    /// Start of click or selection
    ///
    /// The text cursor and selection anchors should be placed at the closest
    /// position to `coord`.
    ///
    /// This corresponds to a mouse click (down). It may be followed by
    /// [`Self::PressMove`] and will be concluded by [`Self::PressMove`] unless
    /// cancelled by calling [`TextInput::stop_selecting`].
    PressStart {
        /// The click coordinate
        coord: Coord,
        /// Whether to clear any prior selection (true unless Shift is held)
        clear: bool,
        /// Number of clicks in sequence (e.g. 2 for double-click)
        repeats: u32,
    },
    /// Drag-motion of pointer
    ///
    /// This is always preceeded by [`Self::PressStart`].
    ///
    /// The text cursor should be placed at the closest position to `coord`,
    /// creating a selection from the anchor placed by [`Self::PressStart`].
    ///
    /// If `repeats > 1` then the selection may be expanded (e.g. to word or
    /// line mode).
    PressMove {
        coord: Coord,
        /// Number of clicks in sequence (e.g. 2 for double-click)
        repeats: u32,
    },
    /// Release of click or touch event
    ///
    /// This may or may not be preceeded by [`Self::PressStart`]: touch events
    /// without motion or sufficient delay to enter selection mode will yield
    /// this variant without a preceeding [`Self::PressStart`].
    ///
    /// This may or may not be preceeded by [`Self::PressMove`].
    ///
    /// Handling should be stateful: if this follows [`Self::PressMove`] then it
    /// terminates selection mode. If this is not preceeded by
    /// [`Self::PressStart`] then the cursor should be placed at the closest
    /// position to `coord`. If this is not preceeded by [`Self::PressMove`]
    /// then it may be considered a "click" action (e.g. to follow a link).
    ///
    /// The widget may wish to request keyboard input focus or IME focus.
    /// The widget should set the primary buffer (Unix).
    PressEnd { coord: Coord },
}

impl TextInput {
    /// Handle input events
    ///
    /// Consumes the following events: `PressStart`, `PressMove`, `PressEnd`,
    /// `Timer(pl)` where `pl == 1<<60 || pl == (1<<60)+1`.
    /// May request press grabs and timer updates.
    ///
    /// May call [`EventCx::set_scroll`] to initiate scrolling.
    pub fn handle(&mut self, cx: &mut EventCx, w_id: Id, event: Event) -> TextInputAction {
        use TextInputAction as Action;
        match event {
            Event::PressStart(press) if press.is_primary() => {
                let mut action = Action::Used;
                let icon = if press.is_touch() {
                    self.phase = TextPhase::PressStart(*press, press.coord());
                    let delay = cx.config().event().touch_select_delay();
                    cx.request_timer(w_id.clone(), TIMER_SELECT, delay);
                    None
                } else if press.is_mouse() {
                    if cx.config_enable_mouse_text_pan() {
                        self.phase = TextPhase::Pan(*press);
                        Some(CursorIcon::Grabbing)
                    } else {
                        self.phase = TextPhase::Cursor(*press, press.coord());
                        action = Action::PressStart {
                            coord: press.coord(),
                            clear: !cx.modifiers().shift_key(),
                            repeats: press.repetitions(),
                        };
                        None
                    }
                } else {
                    unreachable!()
                };
                press
                    .grab(w_id, GrabMode::Grab)
                    .with_opt_icon(icon)
                    .complete(cx);
                action
            }
            Event::PressMove { press, delta } => match self.phase {
                TextPhase::PressStart(source, start_coord) if *press == source => {
                    let delta = press.coord - start_coord;
                    if cx.config_test_pan_thresh(delta) {
                        self.phase = TextPhase::Pan(source);
                        cx.set_scroll(Scroll::Offset(delta.cast()));
                    }
                    Action::Used
                }
                TextPhase::Pan(source) if *press == source => {
                    cx.set_scroll(Scroll::Offset(delta));
                    Action::Used
                }
                TextPhase::Cursor(source, _) if *press == source => {
                    self.phase = TextPhase::Cursor(source, press.coord);
                    Action::PressMove {
                        coord: press.coord,
                        repeats: press.repetitions(),
                    }
                }
                _ => Action::Used,
            },
            Event::PressEnd { press, .. } => match std::mem::take(&mut self.phase) {
                TextPhase::None => Action::Used,
                TextPhase::PressStart(_, coord) => Action::PressEnd { coord },
                TextPhase::Pan(source) => {
                    if *press == source
                        && let Some(vel) = cx.press_velocity(source)
                    {
                        let rest = Vec2::ZERO;
                        cx.set_scroll(Scroll::Kinetic(KineticStart { vel, rest }));
                    }

                    Action::Used
                }
                TextPhase::Cursor(_, coord) => Action::PressEnd { coord },
            },
            Event::Timer(TIMER_SELECT) => match self.phase {
                TextPhase::PressStart(source, coord) => {
                    self.phase = TextPhase::Cursor(source, coord);
                    Action::PressStart {
                        coord,
                        clear: !cx.modifiers().shift_key(),
                        repeats: 1,
                    }
                }
                _ => Action::Unused,
            },
            _ => Action::Unused,
        }
    }

    /// Is there an on-going selection action?
    ///
    /// This is true when the last action delivered was
    /// [`TextInputAction::PressStart`] or [`TextInputAction::PressMove`].
    #[inline]
    pub fn is_selecting(&self) -> bool {
        matches!(&self.phase, TextPhase::Cursor(_, _))
    }

    /// Interrupt an on-going selection action, if any
    pub fn stop_selecting(&mut self) {
        if self.is_selecting() {
            self.phase = TextPhase::None;
        }
    }
}

#[impl_default(ClickPhase::None)]
#[autoimpl(Clone, Debug, PartialEq)]
enum ClickPhase {
    None,
    PressStart(PressSource, Coord), // source, coord
    Pan(PressSource),               // source
}

/// Handles click actions while also allowing scrolling
#[derive(Clone, Debug, Default)]
pub struct ClickInput {
    phase: ClickPhase,
}

/// Result of [`ClickInput::handle`]
#[autoimpl(Clone, Debug)]
pub enum ClickInputAction {
    /// Event is used, no action
    Used,
    /// Event not used
    Unused,
    /// Start of a click / touch event
    ///
    /// This corresponds to a mouse click or touch action before determination
    /// of whether the click action succeeds. It will be concluded by
    /// [`Self::ClickEnd`].
    ClickStart {
        /// The click coordinate
        coord: Coord,
        /// Number of clicks in sequence (e.g. 2 for double-click)
        repeats: u32,
    },
    /// End of a click / touch event
    ///
    /// If `success`, this is a button-release or touch finish; otherwise this
    /// is a cancelled/interrupted grab. "Activation events" (e.g. clicking of a
    /// button or menu item) should only happen on `success`. "Movement events"
    /// such as panning, moving a slider or opening a menu should not be undone
    /// when cancelling: the panned item or slider should be released as is, or
    /// the menu should remain open.
    ClickEnd { coord: Coord, success: bool },
}

impl ClickInput {
    /// Handle input events
    ///
    /// Consumes the following events: `PressStart`, `PressMove`, `PressEnd`.
    /// May request press grabs.
    ///
    /// May call [`EventCx::set_scroll`] to initiate scrolling.
    pub fn handle(&mut self, cx: &mut EventCx, w_id: Id, event: Event) -> ClickInputAction {
        use ClickInputAction as Action;
        match event {
            Event::PressStart(press) if press.is_primary() => {
                let mut action = Action::Used;
                let icon = if cx.config_enable_mouse_text_pan() {
                    self.phase = ClickPhase::Pan(*press);
                    Some(CursorIcon::Grabbing)
                } else {
                    self.phase = ClickPhase::PressStart(*press, press.coord());
                    action = Action::ClickStart {
                        coord: press.coord(),
                        repeats: press.repetitions(),
                    };
                    None
                };
                press
                    .grab(w_id, GrabMode::Grab)
                    .with_opt_icon(icon)
                    .complete(cx);
                action
            }
            Event::PressMove { press, delta } => match self.phase {
                ClickPhase::PressStart(source, start_coord) if *press == source => {
                    let delta = press.coord - start_coord;
                    if cx.config_test_pan_thresh(delta) {
                        self.phase = ClickPhase::Pan(source);
                        cx.set_scroll(Scroll::Offset(delta.cast()));
                    }
                    Action::Used
                }
                ClickPhase::Pan(source) if *press == source => {
                    cx.set_scroll(Scroll::Offset(delta));
                    Action::Used
                }
                _ => Action::Used,
            },
            Event::PressEnd { press, success } => match std::mem::take(&mut self.phase) {
                ClickPhase::PressStart(source, _) if *press == source => Action::ClickEnd {
                    coord: press.coord,
                    success,
                },
                ClickPhase::Pan(source) => {
                    if *press == source
                        && let Some(vel) = cx.press_velocity(source)
                    {
                        let rest = Vec2::ZERO;
                        cx.set_scroll(Scroll::Kinetic(KineticStart { vel, rest }));
                    }

                    Action::Used
                }
                _ => Action::Used,
            },
            _ => Action::Unused,
        }
    }
}
