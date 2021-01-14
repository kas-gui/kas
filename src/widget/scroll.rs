// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use super::ScrollBar;
use kas::draw::{ClipRegion, TextClass};
use kas::event::ScrollDelta::{LineDelta, PixelDelta};
use kas::event::{self, ControlKey, PressSource};
use kas::prelude::*;

/// Logic for a scroll region
///
/// This struct handles some scroll logic. It does not provide scrollbars.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollComponent {
    window_size: Size,
    // TODO: offsets should be a vector, not a coord. This affects a number of conversions.
    max_offset: Coord,
    offset: Coord,
    scroll_rate: f32,
}

impl Default for ScrollComponent {
    #[inline]
    fn default() -> Self {
        ScrollComponent {
            window_size: Size::ZERO,
            max_offset: Coord::ZERO,
            offset: Coord::ZERO,
            scroll_rate: 30.0,
        }
    }
}

impl ScrollComponent {
    /// Get the maximum offset
    ///
    /// Note: the minimum offset is always zero.
    #[inline]
    pub fn max_offset(&self) -> Coord {
        self.max_offset
    }

    /// Get the current offset
    ///
    /// To translate a coordinate from the outer region to a coordinate of the
    /// scrolled region, add this offset.
    #[inline]
    pub fn offset(&self) -> Coord {
        self.offset
    }

    /// Get the window size
    ///
    /// This is the size on the outside: the "window" through which the scrolled
    /// region is viewed (not the application window).
    #[inline]
    pub fn window_size(&self) -> Size {
        self.window_size
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
        self.window_size = window_size;
        self.max_offset = Coord::from(content_size) - Coord::from(window_size);
        self.set_offset(self.offset)
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`TkAction::None`] if the offset is identical to the old offset,
    /// or [`TkAction::RegionMoved`] if the offset changes.
    #[inline]
    pub fn set_offset(&mut self, offset: Coord) -> TkAction {
        let offset = offset.clamp(Coord::ZERO, self.max_offset);
        if offset == self.offset {
            TkAction::None
        } else {
            self.offset = offset;
            TkAction::RegionMoved
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
                *coord = *coord + self.offset;
            }
            Event::PressMove { coord, .. } => {
                *coord = *coord + self.offset;
            }
            Event::PressEnd { coord, .. } => {
                *coord = *coord + self.offset;
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
    /// -   `pos`: the coordinate of the top-left of the scroll area (`rect` is relative to this)
    /// -   returned `Rect`: the focus rect, adjusted for scroll offset; normally this should be
    ///     returned via another [`Response::Focus`]
    /// -   returned `TkAction`: action to pass to the event manager
    #[inline]
    pub fn focus_rect(&mut self, rect: Rect, pos: Coord) -> (Rect, TkAction) {
        // TODO: we want vectors here, not coords (points) and sizes!
        let rel_pos = Coord(rect.pos.0 - pos.0, rect.pos.1 - pos.1);
        let mut offset = self.offset;
        offset = offset.max(rel_pos + rect.size - self.window_size);
        offset = offset.min(rel_pos);
        let action = self.set_offset(offset);
        (rect - self.offset, action)
    }

    /// Use an event to scroll, if possible
    ///
    /// Behaviour on [`Event::PressStart`] is configurable: the closure is called on
    /// this event and should call [`Manager::request_grab`] if the press should
    /// scroll by drag. This allows control of which mouse button(s) are used and
    /// whether any modifiers must be pressed. For example:
    /// ```
    /// # use kas::prelude::*;
    /// # type Msg = ();
    /// fn dummy_event_handler(
    ///     id: WidgetId,
    ///     scroll: &mut ScrollComponent,
    ///     mgr: &mut Manager,
    ///     event: Event
    /// )
    ///     -> Response<Msg>
    /// {
    ///     let (action, response) = scroll.scroll_by_event(event, |source, _, coord| {
    ///         if source.is_primary() {
    ///             let icon = Some(event::CursorIcon::Grabbing);
    ///             mgr.request_grab(id, source, coord, event::GrabMode::Grab, icon);
    ///         }
    ///     });
    ///     *mgr += action;
    ///     response.void_into()
    /// }
    /// ```
    ///
    /// If the returned [`TkAction`] is not `None`, the scroll offset has been
    /// updated. The returned [`Response`] is either `None` or `Unhandled(..)`.
    #[inline]
    pub fn scroll_by_event<PS: FnMut(PressSource, WidgetId, Coord)>(
        &mut self,
        event: Event,
        mut on_press_start: PS,
    ) -> (TkAction, Response<VoidMsg>) {
        let mut action = TkAction::None;
        let mut response = Response::None;

        match event {
            Event::Control(ControlKey::Home) => {
                action = self.set_offset(Coord::ZERO);
            }
            Event::Control(ControlKey::End) => {
                action = self.set_offset(self.max_offset);
            }
            Event::Control(key) => {
                let delta = match key {
                    ControlKey::Left => LineDelta(-1.0, 0.0),
                    ControlKey::Right => LineDelta(1.0, 0.0),
                    ControlKey::Up => LineDelta(0.0, 1.0),
                    ControlKey::Down => LineDelta(0.0, -1.0),
                    ControlKey::PageUp => PixelDelta(Coord(0, self.window_size.1 as i32 / 2)),
                    ControlKey::PageDown => PixelDelta(Coord(0, -(self.window_size.1 as i32 / 2))),
                    key => return (action, Response::Unhandled(Event::Control(key))),
                };

                let d = match delta {
                    LineDelta(x, y) => Coord(
                        (-self.scroll_rate * x) as i32,
                        (self.scroll_rate * y) as i32,
                    ),
                    PixelDelta(d) => d,
                };
                action = self.set_offset(self.offset - d);
            }
            Event::Scroll(delta) => {
                let d = match delta {
                    LineDelta(x, y) => Coord(
                        (-self.scroll_rate * x) as i32,
                        (self.scroll_rate * y) as i32,
                    ),
                    PixelDelta(d) => d,
                };
                action = self.set_offset(self.offset - d);
                if action == TkAction::None {
                    response = Response::Unhandled(Event::Scroll(delta));
                }
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => on_press_start(source, start_id, coord),
            Event::PressMove { delta, .. } => {
                action = self.set_offset(self.offset - delta);
            }
            Event::PressEnd { .. } => (), // consume due to request
            e @ _ => {
                response = Response::Unhandled(e);
            }
        }
        (action, response)
    }
}

/// A scrollable region
///
/// This region supports scrolling via mouse wheel and click/touch drag.
/// Optionally, it can have scroll bars (see [`ScrollRegion::show_bars`] and
/// [`ScrollRegion::with_bars`]).
#[widget(config=noauto)]
#[handler(send=noauto, msg = <W as event::Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[widget_core]
    core: CoreData,
    min_child_size: Size,
    scroll: ScrollComponent,
    auto_bars: bool,
    show_bars: (bool, bool),
    #[widget]
    horiz_bar: ScrollBar<kas::Right>,
    #[widget]
    vert_bar: ScrollBar<kas::Down>,
    #[widget]
    inner: W,
}

impl<W: Widget> ScrollRegion<W> {
    /// Construct a new scroll region around an inner widget
    #[inline]
    pub fn new(inner: W) -> Self {
        ScrollRegion {
            core: Default::default(),
            min_child_size: Size::ZERO,
            scroll: Default::default(),
            auto_bars: false,
            show_bars: (false, false),
            horiz_bar: ScrollBar::new(),
            vert_bar: ScrollBar::new(),
            inner,
        }
    }

    /// Auto-enable bars
    ///
    /// If enabled, this automatically enables/disables scroll bars when
    /// resized.
    ///
    /// This has the side-effect of reserving enough space for scroll bars even
    /// when not required.
    #[inline]
    pub fn with_auto_bars(mut self, enable: bool) -> Self {
        self.auto_bars = enable;
        self
    }

    /// Set which scroll bars are visible
    #[inline]
    pub fn with_bars(mut self, horiz: bool, vert: bool) -> Self {
        self.show_bars = (horiz, vert);
        self
    }

    /// Set which scroll bars are visible
    #[inline]
    pub fn show_bars(&mut self, horiz: bool, vert: bool) {
        self.show_bars = (horiz, vert);
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

    /// Get the maximum scroll offset
    ///
    /// Note: the minimum scroll offset is always zero.
    #[inline]
    pub fn max_scroll_offset(&self) -> Coord {
        self.scroll.max_offset()
    }

    /// Get the current scroll offset
    ///
    /// Contents of the scroll region are translated by this offset (to convert
    /// coordinates from the outer region to the scroll region, add this offset).
    ///
    /// The offset is restricted between [`Coord::ZERO`] and
    /// [`ScrollRegion::max_scroll_offset`].
    #[inline]
    pub fn scroll_offset(&self) -> Coord {
        self.scroll.offset()
    }

    /// Set the scroll offset
    ///
    /// The offset is clamped to the available scroll range.
    /// Returns [`TkAction::None`] if the offset is identical to the old offset,
    /// or a greater action if not identical.
    #[inline]
    pub fn set_scroll_offset(&mut self, offset: Coord) -> TkAction {
        self.scroll.set_offset(offset)
    }
}

impl<W: Widget> WidgetConfig for ScrollRegion<W> {
    fn configure(&mut self, mgr: &mut Manager) {
        mgr.register_nav_fallback(self.id());
    }
}

impl<W: Widget> Layout for ScrollRegion<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut rules = self.inner.size_rules(size_handle, axis);
        if axis.is_horizontal() {
            self.min_child_size.0 = rules.min_size();
        } else {
            self.min_child_size.1 = rules.min_size();
        }
        let line_height = size_handle.line_height(TextClass::Label);
        self.scroll.set_scroll_rate(3.0 * line_height as f32);
        rules.reduce_min_to(line_height);

        if axis.is_horizontal() && (self.auto_bars || self.show_bars.1) {
            rules.append(self.vert_bar.size_rules(size_handle, axis));
        } else if axis.is_vertical() && (self.auto_bars || self.show_bars.0) {
            rules.append(self.horiz_bar.size_rules(size_handle, axis));
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, _: AlignHints) {
        self.core.rect = rect;
        // We use simplified layout code here
        let pos = rect.pos;
        let mut window_size = rect.size;

        let bar_width = (size_handle.scrollbar().0).1;
        if self.auto_bars {
            self.show_bars = (
                self.min_child_size.0 + bar_width > rect.size.0,
                self.min_child_size.1 + bar_width > rect.size.1,
            );
        }
        if self.show_bars.0 {
            window_size.1 -= bar_width;
        }
        if self.show_bars.1 {
            window_size.0 -= bar_width;
        }

        let child_size = window_size.max(self.min_child_size);
        let child_rect = Rect::new(pos, child_size);
        self.inner
            .set_rect(size_handle, child_rect, AlignHints::NONE);
        let _ = self.scroll.set_sizes(window_size, child_size);

        if self.show_bars.0 {
            let pos = Coord(pos.0, pos.1 + window_size.1 as i32);
            let size = Size(window_size.0, bar_width);
            self.horiz_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .horiz_bar
                .set_limits(self.max_scroll_offset().0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(pos.0 + window_size.0 as i32, pos.1);
            let size = Size(bar_width, self.core.rect.size.1);
            self.vert_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .vert_bar
                .set_limits(self.max_scroll_offset().1 as u32, rect.size.1);
        }
    }

    #[inline]
    fn translation(&self, child_index: usize) -> Coord {
        match child_index {
            2 => self.scroll_offset(),
            _ => Coord::ZERO,
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        self.horiz_bar
            .find_id(coord)
            .or_else(|| self.vert_bar.find_id(coord))
            .or_else(|| self.inner.find_id(coord + self.scroll_offset()))
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
        let rect = Rect {
            pos: self.core.rect.pos,
            size: self.scroll.window_size(),
        };
        draw_handle.clip_region(
            rect,
            self.scroll_offset(),
            ClipRegion::Scroll,
            &mut |handle| self.inner.draw(handle, mgr, disabled),
        );
    }
}

impl<W: Widget> event::SendEvent for ScrollRegion<W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let event = if id <= self.horiz_bar.id() {
            match Response::<Self::Msg>::try_from(self.horiz_bar.send(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => event,
                Ok(r) => return r,
                Err(msg) => {
                    *mgr += self.set_scroll_offset(Coord(msg as i32, self.scroll_offset().1));
                    return Response::None;
                }
            }
        } else if id <= self.vert_bar.id() {
            match Response::<Self::Msg>::try_from(self.vert_bar.send(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => event,
                Ok(r) => return r,
                Err(msg) => {
                    *mgr += self.set_scroll_offset(Coord(self.scroll_offset().0, msg as i32));
                    return Response::None;
                }
            }
        } else if id <= self.inner.id() {
            let event = self.scroll.offset_event(event);
            match self.inner.send(mgr, id, event) {
                Response::Unhandled(event) => event,
                Response::Focus(rect) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect.pos);
                    *mgr += action;
                    return Response::Focus(rect);
                }
                r => return r,
            }
        } else {
            event
        };

        let id = self.id();
        let (action, response) = self.scroll.scroll_by_event(event, |source, _, coord| {
            if source.is_primary() {
                let icon = Some(event::CursorIcon::Grabbing);
                mgr.request_grab(id, source, coord, event::GrabMode::Grab, icon);
            }
        });
        if action != TkAction::None {
            *mgr += action
                + self.horiz_bar.set_value(self.scroll_offset().0 as u32)
                + self.vert_bar.set_value(self.scroll_offset().1 as u32);
        }
        response.void_into()
    }
}

impl<W: Widget> std::ops::Deref for ScrollRegion<W> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget> std::ops::DerefMut for ScrollRegion<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
