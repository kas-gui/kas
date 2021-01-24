// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use std::fmt::Debug;

use super::{DragHandle, ScrollRegion};
use kas::{event, prelude::*};

/// A scroll bar
///
/// Scroll bars allow user-input of a value between 0 and a defined maximum,
/// and allow the size of the handle to be specified.
#[handler(send=noauto, msg = u32)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBar<D: Directional> {
    #[widget_core]
    core: CoreData,
    direction: D,
    // Terminology assumes vertical orientation:
    min_handle_len: u32,
    handle_len: u32,
    handle_value: u32, // contract: > 0
    max_value: u32,
    value: u32,
    #[widget]
    handle: DragHandle,
}

impl<D: Directional + Default> ScrollBar<D> {
    /// Construct a scroll bar
    ///
    /// Default values are assumed for all parameters.
    pub fn new() -> Self {
        ScrollBar::new_with_direction(D::default())
    }
}

impl<D: Directional> ScrollBar<D> {
    /// Construct a scroll bar with the given direction
    ///
    /// Default values are assumed for all parameters.
    #[inline]
    pub fn new_with_direction(direction: D) -> Self {
        ScrollBar {
            core: Default::default(),
            direction,
            min_handle_len: 0,
            handle_len: 0,
            handle_value: 1,
            max_value: 0,
            value: 0,
            handle: DragHandle::new(),
        }
    }

    /// Set the initial page length
    ///
    /// See [`ScrollBar::set_limits`].
    #[inline]
    pub fn with_limits(mut self, max_value: u32, handle_value: u32) -> Self {
        let _ = self.set_limits(max_value, handle_value);
        self
    }

    /// Set the initial value
    #[inline]
    pub fn with_value(mut self, value: u32) -> Self {
        self.value = value;
        self
    }

    /// Set the page limits
    ///
    /// The `max_value` parameter specifies the maximum possible value.
    /// (The minimum is always 0.) For a scroll region, this should correspond
    /// to the maximum possible offset.
    ///
    /// The `handle_value` parameter specifies the size of the handle relative to
    /// the maximum value: the handle size relative to the length of the scroll
    /// bar is `handle_value / (max_value + handle_value)`. For a scroll region,
    /// this should correspond to the size of the visible region.
    /// The minimum value is 1.
    ///
    /// The choice of units is not important (e.g. can be pixels or lines),
    /// so long as both parameters use the same units.
    ///
    /// Returns [`TkAction::REDRAW`] if a redraw is required.
    pub fn set_limits(&mut self, max_value: u32, handle_value: u32) -> TkAction {
        // We should gracefully handle zero, though appearance may be wrong.
        self.handle_value = handle_value.max(1);

        self.max_value = max_value;
        self.value = self.value.min(self.max_value);
        self.update_handle()
    }

    /// Read the current max value
    ///
    /// See also the [`ScrollBar::set_limits`] documentation.
    #[inline]
    pub fn max_value(&self) -> u32 {
        self.max_value
    }

    /// Read the current handle value
    ///
    /// See also the [`ScrollBar::set_limits`] documentation.
    #[inline]
    pub fn handle_value(&self) -> u32 {
        self.handle_value
    }

    /// Get the current value
    #[inline]
    pub fn value(&self) -> u32 {
        self.value
    }

    /// Set the value
    pub fn set_value(&mut self, value: u32) -> TkAction {
        let value = value.min(self.max_value);
        if value == self.value {
            TkAction::empty()
        } else {
            self.value = value;
            self.handle.set_offset(self.offset()).1
        }
    }

    #[inline]
    fn len(&self) -> u32 {
        match self.direction.is_vertical() {
            false => self.core.rect.size.0,
            true => self.core.rect.size.1,
        }
    }

    fn update_handle(&mut self) -> TkAction {
        let len = self.len();
        let total = self.max_value as u64 + self.handle_value as u64;
        let handle_len = self.handle_value as u64 * len as u64 / total;
        self.handle_len = (handle_len as u32).max(self.min_handle_len).min(len);
        let mut size = self.core.rect.size;
        if self.direction.is_horizontal() {
            size.0 = self.handle_len;
        } else {
            size.1 = self.handle_len;
        }
        self.handle.set_size_and_offset(size, self.offset())
    }

    // translate value to offset in local coordinates
    fn offset(&self) -> Coord {
        let len = self.len() - self.handle_len;
        let lhs = self.value as u64 * len as u64;
        let rhs = self.max_value as u64;
        let mut pos = if rhs == 0 {
            0
        } else {
            (((lhs + (rhs / 2)) / rhs) as u32).min(len)
        };
        if self.direction.is_reversed() {
            pos = len - pos;
        }
        match self.direction.is_vertical() {
            false => Coord(pos as i32, 0),
            true => Coord(0, pos as i32),
        }
    }

    // true if not equal to old value
    fn set_offset(&mut self, offset: Coord) -> bool {
        let len = self.len() - self.handle_len;
        let mut offset = match self.direction.is_vertical() {
            false => offset.0,
            true => offset.1,
        } as u32;
        if self.direction.is_reversed() {
            offset = len - offset;
        }

        let lhs = offset as u64 * self.max_value as u64;
        let rhs = len as u64;
        if rhs == 0 {
            debug_assert_eq!(self.value, 0);
            return false;
        }
        let value = ((lhs + (rhs / 2)) / rhs) as u32;
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            return true;
        }
        false
    }
}

impl<D: Directional> Layout for ScrollBar<D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let (size, min_len) = size_handle.scrollbar();
        self.min_handle_len = size.0;
        let margins = (0, 0);
        if self.direction.is_vertical() == axis.is_vertical() {
            SizeRules::new(min_len, min_len, margins, StretchPolicy::HighUtility)
        } else {
            SizeRules::fixed(size.1, margins)
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        self.handle.set_rect(size_handle, rect, align);
        let _ = self.update_handle();
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        self.handle.find_id(coord).or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let dir = self.direction.as_direction();
        let state = self.handle.input_state(mgr, disabled);
        draw_handle.scrollbar(self.core.rect, self.handle.rect(), dir, state);
    }
}

impl<D: Directional> event::SendEvent for ScrollBar<D> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let offset = if id <= self.handle.id() {
            match self.handle.send(mgr, id, event).try_into() {
                Ok(res) => return res,
                Err(offset) => offset,
            }
        } else {
            match event {
                Event::PressStart { source, coord, .. } => {
                    self.handle.handle_press_on_track(mgr, source, coord)
                }
                ev @ _ => return Response::Unhandled(ev),
            }
        };

        if self.set_offset(offset) {
            mgr.redraw(self.handle.id());
            Response::Msg(self.value)
        } else {
            Response::None
        }
    }
}

/// Additional functionality on scrollable widgets
///
/// This may be used to add controls via the [`ScrollBars`] wrapper.
///
/// The implementing widget may use event handlers to scroll itself (e.g. in
/// reaction to a mouse wheel or touch-drag), but when doing so should emit
/// [`Response::Focus`] to notify any wrapper of the new position (usually with
/// `Response::Focus(self.rect())`).
pub trait ScrollWidget: Widget {
    /// Given size `size`, returns whether `(horiz, vert)` scrolling is required
    fn scroll_axes(&self, size: Size) -> (bool, bool);

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    fn max_scroll_offset(&self) -> Coord;

    /// Get the current scroll offset
    ///
    /// Contents of the scroll region are translated by this offset (to convert
    /// coordinates from the outer region to the scroll region, add this offset).
    ///
    /// The offset is restricted between [`Coord::ZERO`] and
    /// [`ScrollRegion::max_scroll_offset`].
    fn scroll_offset(&self) -> Coord;

    /// Set the scroll offset
    ///
    /// This may be used for programmatic scrolling, e.g. by a wrapping widget
    /// with scroll controls. Note that calling this method directly on the
    /// scrolling widget will not update any controls in a wrapping widget.
    ///
    /// The offset is clamped to the available scroll range and applied. The
    /// resulting offset is returned.
    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Coord) -> Coord;
}

/// A scrollable region with bars
///
/// This is merely a typedef
pub type ScrollBarRegion<W> = ScrollBars<ScrollRegion<W>>;

/// Scrollbar controls
///
/// This is a wrapper adding scrollbar controls around a child. Note that this
/// widget does not enable scrolling; see [`ScrollRegion`] for that.
/// This region supports scrolling via mouse wheel and click/touch drag.
#[widget(config=noauto)]
#[handler(send=noauto, msg = <W as event::Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBars<W: ScrollWidget> {
    #[widget_core]
    core: CoreData,
    auto_bars: bool,
    show_bars: (bool, bool),
    #[widget]
    horiz_bar: ScrollBar<kas::Right>,
    #[widget]
    vert_bar: ScrollBar<kas::Down>,
    #[widget]
    inner: W,
}

impl<W: Widget> ScrollBars<ScrollRegion<W>> {
    /// Construct a `ScrollBars<ScrollRegion<W>>`
    ///
    /// This is a convenience constructor.
    #[inline]
    pub fn new2(inner: W) -> Self {
        ScrollBars::new(ScrollRegion::new(inner))
    }
}

impl<W: ScrollWidget> ScrollBars<W> {
    /// Construct
    ///
    /// By default scrollbars are automatically enabled based on requirements.
    /// See [`ScrollBars::with_auto_bars`] and [`ScrollBars::with_bars`].
    #[inline]
    pub fn new(inner: W) -> Self {
        ScrollBars {
            core: Default::default(),
            auto_bars: true,
            show_bars: (false, false),
            horiz_bar: ScrollBar::new(),
            vert_bar: ScrollBar::new(),
            inner,
        }
    }

    /// Auto-enable bars
    ///
    /// If enabled (default), this automatically enables/disables scroll bars
    /// as required when resized.
    ///
    /// This has the side-effect of reserving enough space for scroll bars even
    /// when not required.
    #[inline]
    pub fn with_auto_bars(mut self, enable: bool) -> Self {
        self.auto_bars = enable;
        self
    }

    /// Set which scroll bars are visible
    ///
    /// Calling this method also disables automatic scroll bars.
    #[inline]
    pub fn with_bars(mut self, horiz: bool, vert: bool) -> Self {
        self.auto_bars = false;
        self.show_bars = (horiz, vert);
        self
    }

    /// Set which scroll bars are visible
    ///
    /// Calling this method also disables automatic scroll bars.
    #[inline]
    pub fn set_bars(&mut self, horiz: bool, vert: bool) {
        self.auto_bars = false;
        self.show_bars = (horiz, vert);
    }

    /// Query which scroll bars are visible
    ///
    /// Returns `(horiz, vert)` tuple.
    #[inline]
    pub fn bars(&self) -> (bool, bool) {
        self.show_bars
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

impl<W: ScrollWidget> ScrollWidget for ScrollBars<W> {
    fn scroll_axes(&self, size: Size) -> (bool, bool) {
        self.inner.scroll_axes(size)
    }
    fn max_scroll_offset(&self) -> Coord {
        self.inner.max_scroll_offset()
    }
    fn scroll_offset(&self) -> Coord {
        self.inner.scroll_offset()
    }
    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Coord) -> Coord {
        let offset = self.inner.set_scroll_offset(mgr, offset);
        *mgr |=
            self.horiz_bar.set_value(offset.0 as u32) | self.vert_bar.set_value(offset.1 as u32);
        offset
    }
}

impl<W: ScrollWidget> WidgetConfig for ScrollBars<W> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.register_nav_fallback(self.id());
    }
}

impl<W: ScrollWidget> Layout for ScrollBars<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut rules = self.inner.size_rules(size_handle, axis);
        if axis.is_horizontal() && (self.auto_bars || self.show_bars.1) {
            rules.append(self.vert_bar.size_rules(size_handle, axis));
        } else if axis.is_vertical() && (self.auto_bars || self.show_bars.0) {
            rules.append(self.horiz_bar.size_rules(size_handle, axis));
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let pos = rect.pos;
        let mut child_size = rect.size;

        let bar_width = (size_handle.scrollbar().0).1;
        if self.auto_bars {
            child_size -= Size(bar_width, bar_width);
            self.show_bars = self.inner.scroll_axes(child_size);
        } else {
            if self.show_bars.0 {
                child_size.1 -= bar_width;
            }
            if self.show_bars.1 {
                child_size.0 -= bar_width;
            }
        }

        let child_rect = Rect::new(pos, child_size);
        self.inner.set_rect(size_handle, child_rect, align);
        let max_scroll_offset = self.inner.max_scroll_offset();

        if self.show_bars.0 {
            let pos = Coord(pos.0, pos.1 + child_size.1 as i32);
            let size = Size(child_size.0, bar_width);
            self.horiz_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .horiz_bar
                .set_limits(max_scroll_offset.0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(pos.0 + child_size.0 as i32, pos.1);
            let size = Size(bar_width, self.core.rect.size.1);
            self.vert_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .vert_bar
                .set_limits(max_scroll_offset.1 as u32, rect.size.1);
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        self.horiz_bar
            .find_id(coord)
            .or_else(|| self.vert_bar.find_id(coord))
            .or_else(|| self.inner.find_id(coord))
            .or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        if self.show_bars.0 {
            self.horiz_bar.draw(draw_handle, mgr, disabled);
        }
        if self.show_bars.1 {
            self.vert_bar.draw(draw_handle, mgr, disabled);
        }
        self.inner.draw(draw_handle, mgr, disabled);
    }
}

impl<W: ScrollWidget> event::SendEvent for ScrollBars<W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.horiz_bar.id() {
            self.horiz_bar
                .send(mgr, id, event)
                .try_into()
                .unwrap_or_else(|msg| {
                    let offset = Coord(msg as i32, self.inner.scroll_offset().1);
                    self.inner.set_scroll_offset(mgr, offset);
                    Response::None
                })
        } else if id <= self.vert_bar.id() {
            self.vert_bar
                .send(mgr, id, event)
                .try_into()
                .unwrap_or_else(|msg| {
                    let offset = Coord(self.inner.scroll_offset().0, msg as i32);
                    self.inner.set_scroll_offset(mgr, offset);
                    Response::None
                })
        } else if id <= self.inner.id() {
            match self.inner.send(mgr, id, event) {
                Response::Focus(rect) => {
                    // We assume that the scrollable inner already updated its
                    // offset; we just update the bar positions
                    let offset = self.inner.scroll_offset();
                    *mgr |= self.horiz_bar.set_value(offset.0 as u32)
                        | self.vert_bar.set_value(offset.1 as u32);
                    Response::Focus(rect)
                }
                r => r,
            }
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            self.handle(mgr, event)
        }
    }
}

impl<W: ScrollWidget> std::ops::Deref for ScrollBars<W> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: ScrollWidget> std::ops::DerefMut for ScrollBars<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
