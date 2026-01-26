// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use super::{GripMsg, GripPart};
use kas::event::TimerHandle;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;

/// Scroll bar mode
///
/// The default value is [`ScrollBarMode::Auto`].
#[impl_default(ScrollBarMode::Auto)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScrollBarMode {
    /// Automatically enable/disable scroll bars as required when resized.
    ///
    /// This has the side-effect of reserving enough space for scroll bars even
    /// when not required.
    Auto,
    /// Each scroll bar has fixed visibility.
    ///
    /// Parameters: `(horiz_is_visible, vert_is_visible)`.
    Fixed(bool, bool),
    /// Enabled scroll bars float over content and are only drawn on mouse over.
    /// Disabled scroll bars are fully hidden.
    ///
    /// Parameters: `(horiz_is_enabled, vert_is_enabled)`.
    Invisible(bool, bool),
}

/// Message from a [`ScrollBar`]
#[derive(Copy, Clone, Debug)]
pub struct ScrollBarMsg(pub i32);

const TIMER_HIDE: TimerHandle = TimerHandle::new(0, false);

#[impl_self]
mod ScrollBar {
    /// A scroll bar
    ///
    /// Scroll bars allow user-input of a value between 0 and a defined maximum,
    /// and allow the size of the grip to be specified.
    ///
    /// # Messages
    ///
    /// On value change, pushes a value of type [`ScrollBarMsg`].
    ///
    /// # Layout
    ///
    /// It is safe to not call `size_rules` before `set_rect` for this type.
    #[derive(Debug, Default)]
    #[widget]
    pub struct ScrollBar<D: Directional = Direction> {
        core: widget_core!(),
        direction: D,
        // Terminology assumes vertical orientation:
        min_grip_len: i32, // units: px
        grip_len: i32,     // units: px
        // grip_size, max_value and value are all in arbitrary (user-provided) units:
        grip_size: i32, // contract: > 0; relative to max_value
        max_value: i32,
        value: i32,
        invisible: bool,
        is_under_mouse: bool,
        force_visible: bool,
        #[widget]
        grip: GripPart,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a scroll bar
        ///
        /// Default values are assumed for all parameters.
        #[inline]
        pub fn new() -> Self {
            ScrollBar::new_dir(D::default())
        }
    }
    impl ScrollBar<kas::dir::Down> {
        /// Construct a scroll bar (vertical)
        ///
        /// Default values are assumed for all parameters.
        pub fn down() -> Self {
            ScrollBar::new()
        }
    }
    impl ScrollBar<kas::dir::Right> {
        /// Construct a scroll bar (horizontal)
        ///
        /// Default values are assumed for all parameters.
        pub fn right() -> Self {
            ScrollBar::new()
        }
    }

    impl Self {
        /// Construct a scroll bar with the given direction
        ///
        /// Default values are assumed for all parameters.
        #[inline]
        pub fn new_dir(direction: D) -> Self {
            ScrollBar {
                core: Default::default(),
                direction,
                min_grip_len: 0,
                grip_len: 0,
                grip_size: 1,
                max_value: 0,
                value: 0,
                invisible: false,
                is_under_mouse: false,
                force_visible: false,
                grip: GripPart::new(),
            }
        }

        /// Get the scroll bar's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Get whether the scroll bar is set as invisible
        ///
        /// This refers to the property set by [`Self::set_invisible`] / [`Self::with_invisible`],
        /// not [`Self::currently_visible`].
        #[inline]
        pub fn is_invisible(&self) -> bool {
            self.invisible
        }

        /// Set invisible property
        ///
        /// An "invisible" scroll bar is only drawn on mouse-over
        #[inline]
        pub fn set_invisible(&mut self, invisible: bool) {
            self.invisible = invisible;
        }

        /// Set invisible property (inline)
        ///
        /// An "invisible" scroll bar is only drawn on mouse-over
        #[inline]
        pub fn with_invisible(mut self, invisible: bool) -> Self {
            self.invisible = invisible;
            self
        }

        /// Set the initial page length
        ///
        /// See [`ScrollBar::set_limits`].
        #[inline]
        #[must_use]
        pub fn with_limits(mut self, max_value: i32, grip_size: i32) -> Self {
            // We should gracefully handle zero, though appearance may be wrong.
            self.grip_size = grip_size.max(1);

            self.max_value = max_value.max(0);
            self.value = self.value.clamp(0, self.max_value);
            self
        }

        /// Set the initial value
        #[inline]
        #[must_use]
        pub fn with_value(mut self, value: i32) -> Self {
            self.value = value.clamp(0, self.max_value);
            self
        }

        /// Set the page limits
        ///
        /// The `max_value` parameter specifies the maximum possible value.
        /// (The minimum is always 0.) For a scroll region, this should correspond
        /// to the maximum possible offset.
        ///
        /// The `grip_size` parameter specifies the size of the grip relative to
        /// the maximum value: the grip size relative to the length of the scroll
        /// bar is `grip_size / (max_value + grip_size)`. For a scroll region,
        /// this should correspond to the size of the visible region.
        /// The minimum value is 1.
        ///
        /// The choice of units is not important (e.g. can be pixels or lines),
        /// so long as both parameters use the same units.
        pub fn set_limits(&mut self, cx: &mut EventState, max_value: i32, grip_size: i32) {
            // We should gracefully handle zero, though appearance may be wrong.
            self.grip_size = grip_size.max(1);

            self.max_value = max_value.max(0);
            self.value = self.value.clamp(0, self.max_value);
            self.update_widgets(cx);
        }

        /// Read the current max value
        ///
        /// See also the [`ScrollBar::set_limits`] documentation.
        #[inline]
        pub fn max_value(&self) -> i32 {
            self.max_value
        }

        /// Read the current grip value
        ///
        /// See also the [`ScrollBar::set_limits`] documentation.
        #[inline]
        pub fn grip_size(&self) -> i32 {
            self.grip_size
        }

        /// Get the current value
        #[inline]
        pub fn value(&self) -> i32 {
            self.value
        }

        /// Set the value
        ///
        /// Returns true if the value changes.
        pub fn set_value(&mut self, cx: &mut EventState, value: i32) -> bool {
            let value = value.clamp(0, self.max_value);
            let changed = value != self.value;
            if changed {
                self.value = value;
                self.grip.set_offset(cx, self.offset());
            }
            if !self.is_under_mouse {
                self.force_visible = true;
                let delay = cx.config().event().touch_select_delay();
                cx.request_timer(self.id(), TIMER_HIDE, delay);
            }
            changed
        }

        #[inline]
        fn bar_len(&self) -> i32 {
            match self.direction.is_vertical() {
                false => self.rect().size.0,
                true => self.rect().size.1,
            }
        }

        fn update_widgets(&mut self, cx: &mut EventState) {
            let len = self.bar_len();
            let total = 1i64.max(i64::from(self.max_value) + i64::from(self.grip_size));
            let grip_len = i64::from(self.grip_size) * i64::conv(len) / total;
            self.grip_len = i32::conv(grip_len).max(self.min_grip_len).min(len);
            let mut size = self.rect().size;
            size.set_component(self.direction, self.grip_len);
            self.grip.set_size(size);
            self.grip.set_offset(cx, self.offset());
        }

        // translate value to offset in local coordinates
        fn offset(&self) -> Offset {
            let len = self.bar_len() - self.grip_len;
            let lhs = i64::from(self.value) * i64::conv(len);
            let rhs = i64::from(self.max_value);
            let mut pos = if rhs == 0 {
                0
            } else {
                i32::conv((lhs + (rhs / 2)) / rhs).min(len)
            };
            if self.direction.is_reversed() {
                pos = len - pos;
            }
            match self.direction.is_vertical() {
                false => Offset(pos, 0),
                true => Offset(0, pos),
            }
        }

        // true if not equal to old value
        fn apply_grip_offset(&mut self, cx: &mut EventCx, offset: Offset) {
            let offset = self.grip.set_offset(cx, offset);

            let len = self.bar_len() - self.grip_len;
            let mut offset = match self.direction.is_vertical() {
                false => offset.0,
                true => offset.1,
            };
            if self.direction.is_reversed() {
                offset = len - offset;
            }

            let lhs = i64::from(offset) * i64::from(self.max_value);
            let rhs = i64::conv(len);
            if rhs == 0 {
                debug_assert_eq!(self.value, 0);
                return;
            }
            let value = i32::conv((lhs + (rhs / 2)) / rhs);
            if self.set_value(cx, value) {
                cx.push(ScrollBarMsg(value));
            }
        }

        /// Get whether the scroll bar is currently visible
        ///
        /// This property may change frequently. The method is intended only to
        /// allow omitting draw calls while the scroll bar is not visible, since
        /// these draw calls may require use of an additional draw pass to allow
        /// an "invisible" scroll bar to be drawn over content.
        #[inline]
        pub fn currently_visible(&self, ev_state: &EventState) -> bool {
            !self.invisible
                || (self.max_value != 0 && self.force_visible)
                || ev_state.is_depressed(self.grip.id_ref())
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let _ = self.grip.size_rules(cx, axis);
            cx.feature(Feature::ScrollBar(self.direction()), axis)
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, hints: AlignHints) {
            let align = match self.direction.is_vertical() {
                false => AlignPair::new(Align::Stretch, hints.vert.unwrap_or(Align::Center)),
                true => AlignPair::new(hints.horiz.unwrap_or(Align::Center), Align::Stretch),
            };
            let rect = cx.align_feature(Feature::ScrollBar(self.direction()), rect, align);
            self.core.set_rect(rect);
            self.grip.set_track(rect);

            // We call grip.set_rect only for compliance with the widget model:
            self.grip.set_rect(cx, Rect::ZERO, AlignHints::NONE);

            self.min_grip_len = cx.grip_len();
            self.update_widgets(cx);
        }

        #[inline]
        fn draw(&self, mut draw: DrawCx) {
            if self.currently_visible(draw.ev_state()) {
                let dir = self.direction.as_direction();
                draw.scroll_bar(self.rect(), &self.grip, dir);
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollBar {
                direction: self.direction.as_direction(),
                value: self.value,
                max_value: self.max_value,
            }
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = ();

        fn probe(&self, coord: Coord) -> Id {
            if self.invisible && self.max_value == 0 {
                return self.id();
            }
            self.grip.try_probe(coord).unwrap_or_else(|| self.id())
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(TIMER_HIDE) => {
                    if !self.is_under_mouse {
                        self.force_visible = false;
                        cx.redraw();
                    }
                    Used
                }
                Event::PressStart(press) => {
                    let offset = self.grip.handle_press_on_track(cx, &press);
                    self.apply_grip_offset(cx, offset);
                    Used
                }
                Event::MouseOver(true) => {
                    self.is_under_mouse = true;
                    self.force_visible = true;
                    cx.redraw();
                    Used
                }
                Event::MouseOver(false) => {
                    self.is_under_mouse = false;
                    let delay = cx.config().event().touch_select_delay();
                    cx.request_timer(self.id(), TIMER_HIDE, delay);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(GripMsg::PressMove(offset)) = cx.try_pop() {
                self.apply_grip_offset(cx, offset);
            }
        }
    }
}
