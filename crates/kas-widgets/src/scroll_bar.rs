// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use super::{GripMsg, GripPart, ScrollRegion};
use kas::event::Scroll;
use kas::prelude::*;
use kas::theme::Feature;
use std::fmt::Debug;

/// Message from a [`ScrollBar`]
#[derive(Copy, Clone, Debug)]
pub struct ScrollMsg(pub i32);

impl_scope! {
    /// A scroll bar
    ///
    /// Scroll bars allow user-input of a value between 0 and a defined maximum,
    /// and allow the size of the grip to be specified.
    ///
    /// # Messages
    ///
    /// On value change, pushes a value of type [`ScrollMsg`].
    ///
    /// # Layout
    ///
    /// It is safe to not call `size_rules` before `set_rect` for this type.
    #[derive(Clone, Debug, Default)]
    #[widget(hover_highlight = true;)]
    pub struct ScrollBar<D: Directional = Direction> {
        core: widget_core!(),
        direction: D,
        // Terminology assumes vertical orientation:
        min_grip_len: i32, // units: px
        grip_len: i32, // units: px
        // grip_size, max_value and value are all in arbitrary (user-provided) units:
        grip_size: i32, // contract: > 0; relative to max_value
        max_value: i32,
        value: i32,
        invisible: bool,
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
                force_visible: false,
                grip: GripPart::new(),
            }
        }

        /// Get the scroll bar's direction
        #[inline]
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set invisible property
        ///
        /// An "invisible" scroll bar is only drawn on mouse-hover
        #[inline]
        pub fn set_invisible(&mut self, invisible: bool) {
            self.invisible = invisible;
        }

        /// Set invisible property (inline)
        ///
        /// An "invisible" scroll bar is only drawn on mouse-hover
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
            let _ = self.set_limits(max_value, grip_size);
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
        ///
        /// Returns [`Action::REDRAW`] if a redraw is required.
        pub fn set_limits(&mut self, max_value: i32, grip_size: i32) -> Action {
            // We should gracefully handle zero, though appearance may be wrong.
            self.grip_size = grip_size.max(1);

            self.max_value = max_value.max(0);
            self.value = self.value.clamp(0, self.max_value);
            self.update_widgets()
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
                let action = self.grip.set_offset(self.offset()).1;
                cx.action(&self, action);
            }
            self.force_visible(cx);
            changed
        }

        fn force_visible(&mut self, cx: &mut EventState) {
            self.force_visible = true;
            let delay = cx.config().event().touch_select_delay();
            cx.request_timer(self.id(), 0, delay);
        }

        #[inline]
        fn bar_len(&self) -> i32 {
            match self.direction.is_vertical() {
                false => self.core.rect.size.0,
                true => self.core.rect.size.1,
            }
        }

        fn update_widgets(&mut self) -> Action {
            let len = self.bar_len();
            let total = 1i64.max(i64::from(self.max_value) + i64::from(self.grip_size));
            let grip_len = i64::from(self.grip_size) * i64::conv(len) / total;
            self.grip_len = i32::conv(grip_len).max(self.min_grip_len).min(len);
            let mut size = self.core.rect.size;
            size.set_component(self.direction, self.grip_len);
            self.grip.set_size(size);
            self.grip.set_offset(self.offset()).1
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
            let (offset, action) = self.grip.set_offset(offset);
            cx.action(&self, action);

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
                cx.push(ScrollMsg(value));
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let _ = self.grip.size_rules(sizer.re(), axis);
            sizer.feature(Feature::ScrollBar(self.direction()), axis)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            let align = match self.direction.is_vertical() {
                false => AlignPair::new(Align::Stretch, hints.vert.unwrap_or(Align::Center)),
                true => AlignPair::new(hints.horiz.unwrap_or(Align::Center), Align::Stretch),
            };
            let rect = cx.align_feature(Feature::ScrollBar(self.direction()), rect, align);
            widget_set_rect!(rect);
            self.grip.set_track(rect);

            // We call grip.set_rect only for compliance with the widget model:
            self.grip.set_rect(cx, Rect::ZERO, AlignHints::NONE);

            self.min_grip_len = cx.size_cx().grip_len();
            let _ = self.update_widgets();
        }

        fn draw(&mut self, mut draw: DrawCx) {
            if draw.ev_state().is_hovered_recursive(self.id_ref()) {
                self.force_visible(draw.ev_state());
            }

            if !self.invisible
                || (self.max_value != 0 && self.force_visible)
                || draw.ev_state().is_depressed(self.grip.id_ref())
            {
                let dir = self.direction.as_direction();
                draw.scroll_bar(self.rect(), &self.grip, dir);
            }
        }
    }

    impl Events for Self {
        type Data = ();

        fn probe(&mut self, coord: Coord) -> Id {
            if self.invisible && self.max_value == 0 {
                return self.id();
            }
            self.grip.try_probe(coord).unwrap_or_else(|| self.id())
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(_) => {
                    self.force_visible = false;
                    cx.redraw(self);
                    Used
                }
                Event::PressStart { press } => {
                    let offset = self.grip.handle_press_on_track(cx, &press);
                    self.apply_grip_offset(cx, offset);
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

impl_scope! {
    /// Scroll bar controls
    ///
    /// This is a wrapper adding scroll bar controls around a child. Note that this
    /// widget does not enable scrolling; see [`ScrollBarRegion`] for that.
    ///
    /// Scroll bar positioning does not respect the inner widget's margins, since
    /// the result looks poor when content is scrolled. Instead the content should
    /// force internal margins by wrapping contents with a (zero-sized) frame.
    /// [`ScrollRegion`] already does this.
    #[autoimpl(class_traits using self.inner where W: trait)]
    #[impl_default(where W: trait)]
    #[derive(Clone, Debug)]
    #[widget {
        Data = W::Data;
    }]
    pub struct ScrollBars<W: Scrollable + Widget> {
        core: widget_core!(),
        mode: ScrollBarMode,
        show_bars: (bool, bool), // set by user (or set_rect when mode == Auto)
        #[widget(&())]
        horiz_bar: ScrollBar<kas::dir::Right>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct
        ///
        /// By default scroll bars are automatically enabled based on requirements.
        /// Use the [`HasScrollBars`] trait to adjust this behaviour.
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollBars {
                core: Default::default(),
                mode: ScrollBarMode::Auto,
                show_bars: (false, false),
                horiz_bar: ScrollBar::new(),
                vert_bar: ScrollBar::new(),
                inner,
            }
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner(&self) -> &W {
            &self.inner
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner_mut(&mut self) -> &mut W {
            &mut self.inner
        }
    }

    impl HasScrollBars for Self {
        fn get_mode(&self) -> ScrollBarMode {
            self.mode
        }
        fn set_mode(&mut self, mode: ScrollBarMode) -> Action {
            self.mode = mode;
            let invisible = mode == ScrollBarMode::Invisible;
            self.horiz_bar.set_invisible(invisible);
            self.vert_bar.set_invisible(invisible);
            Action::RESIZE
        }

        fn get_visible_bars(&self) -> (bool, bool) {
            self.show_bars
        }
        fn set_visible_bars(&mut self, bars: (bool, bool)) -> Action {
            self.show_bars = bars;
            Action::RESIZE
        }
    }

    impl Scrollable for Self {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.inner.scroll_axes(size)
        }
        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.inner.max_scroll_offset()
        }
        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.inner.scroll_offset()
        }
        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let offset = self.inner.set_scroll_offset(cx, offset);
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
            offset
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut rules = self.inner.size_rules(sizer.re(), axis);
            let vert_rules = self.vert_bar.size_rules(sizer.re(), axis);
            let horiz_rules = self.horiz_bar.size_rules(sizer.re(), axis);
            let (use_horiz, use_vert) = match self.mode {
                ScrollBarMode::Fixed => self.show_bars,
                ScrollBarMode::Auto => (true, true),
                ScrollBarMode::Invisible => (false, false),
            };
            if axis.is_horizontal() && use_horiz {
                rules.append(vert_rules);
            } else if axis.is_vertical() && use_vert {
                rules.append(horiz_rules);
            }
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            let pos = rect.pos;
            let mut child_size = rect.size;

            let bar_width = cx.size_cx().scroll_bar_width();
            if self.mode == ScrollBarMode::Auto {
                self.show_bars = self.inner.scroll_axes(child_size);
            }
            if self.show_bars.0 && !self.horiz_bar.invisible {
                child_size.1 -= bar_width;
            }
            if self.show_bars.1 && !self.vert_bar.invisible {
                child_size.0 -= bar_width;
            }

            let child_rect = Rect::new(pos, child_size);
            self.inner.set_rect(cx, child_rect, hints);
            let max_scroll_offset = self.inner.max_scroll_offset();

            if self.show_bars.0 {
                let pos = Coord(pos.0, rect.pos2().1 - bar_width);
                let size = Size::new(child_size.0, bar_width);
                self.horiz_bar.set_rect(cx, Rect { pos, size }, AlignHints::NONE);
                let _ = self.horiz_bar.set_limits(max_scroll_offset.0, rect.size.0);
            } else {
                self.horiz_bar.set_rect(cx, Rect::ZERO, AlignHints::NONE);
            }

            if self.show_bars.1 {
                let pos = Coord(rect.pos2().0 - bar_width, pos.1);
                let size = Size::new(bar_width, self.core.rect.size.1);
                self.vert_bar.set_rect(cx, Rect { pos, size }, AlignHints::NONE);
                let _ = self.vert_bar.set_limits(max_scroll_offset.1, rect.size.1);
            } else {
                self.vert_bar.set_rect(cx, Rect::ZERO, AlignHints::NONE);
            }
        }

        fn draw(&mut self, mut draw: DrawCx) {
            self.inner.draw(draw.re());
            draw.with_pass(|mut draw| {
                if self.show_bars.0 {
                    self.horiz_bar.draw(draw.re());
                }
                if self.show_bars.1 {
                    self.vert_bar.draw(draw.re());
                }
            });
        }
    }

    impl Events for Self {
        fn probe(&mut self, coord: Coord) -> Id {
            self.vert_bar.try_probe(coord)
                .or_else(|| self.horiz_bar.try_probe(coord))
                .or_else(|| self.inner.try_probe(coord))
                .unwrap_or_else(|| self.id())
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            let index = cx.last_child();
            if index == Some(widget_index![self.horiz_bar]) {
                if let Some(ScrollMsg(x)) = cx.try_pop() {
                    let offset = Offset(x, self.inner.scroll_offset().1);
                    self.inner.set_scroll_offset(cx, offset);
                }
            } else if index == Some(widget_index![self.vert_bar]) {
                if let Some(ScrollMsg(y)) = cx.try_pop() {
                    let offset = Offset(self.inner.scroll_offset().0, y);
                    self.inner.set_scroll_offset(cx, offset);
                }
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, _: Scroll) {
            // We assume the inner already updated its positions; this is just to set bars
            let offset = self.inner.scroll_offset();
            self.horiz_bar.set_value(cx, offset.0);
            self.vert_bar.set_value(cx, offset.1);
        }
    }
}

impl_scope! {
    /// A scrollable region with bars
    ///
    /// This is essentially a `ScrollBars<ScrollRegion<W>>`:
    /// [`ScrollRegion`] handles the actual scrolling and wheel/touch events,
    /// while [`ScrollBars`] adds scroll bar controls.
    ///
    /// Use the [`HasScrollBars`] trait to adjust scroll bar behaviour.
    #[autoimpl(Deref, DerefMut, HasScrollBars, Scrollable using self.0)]
    #[autoimpl(class_traits using self.0 where W: trait)]
    #[derive(Clone, Debug, Default)]
    #[widget{
        derive = self.0;
    }]
    pub struct ScrollBarRegion<W: Widget>(ScrollBars<ScrollRegion<W>>);

    impl Self {
        /// Construct a `ScrollBarRegion<W>`
        #[inline]
        pub fn new(inner: W) -> Self {
            ScrollBarRegion(ScrollBars::new(ScrollRegion::new(inner)))
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner(&self) -> &W {
            self.0.inner.inner()
        }

        /// Access inner widget directly
        #[inline]
        pub fn inner_mut(&mut self) -> &mut W {
            self.0.inner.inner_mut()
        }
    }
}
