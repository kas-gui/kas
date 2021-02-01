// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use super::ScrollWidget;
use kas::draw::{ClipRegion, TextClass};
use kas::event::ScrollDelta::{LineDelta, PixelDelta};
use kas::event::{self, ControlKey, PressSource};
use kas::prelude::*;
use std::fmt::Debug;

/// Logic for a scroll region
///
/// This struct handles some scroll logic. It does not provide scrollbars.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollComponent {
    max_offset: Offset,
    offset: Offset,
    scroll_rate: f32,
}

impl Default for ScrollComponent {
    #[inline]
    fn default() -> Self {
        ScrollComponent {
            max_offset: Offset::ZERO,
            offset: Offset::ZERO,
            scroll_rate: 30.0,
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
    /// Behaviour on [`Event::PressStart`] is configurable: the closure is called on
    /// this event and should call [`Manager::request_grab`] if the press should
    /// scroll by drag. This allows control of which mouse button(s) are used and
    /// whether any modifiers must be pressed. For example:
    /// ```
    /// # use kas::prelude::*;
    /// # type Msg = ();
    /// fn dummy_event_handler(
    ///     id: WidgetId,
    ///     scroll: &mut kas::widget::ScrollComponent,
    ///     mgr: &mut Manager,
    ///     event: Event
    /// )
    ///     -> Response<Msg>
    /// {
    ///     let window_size = Size(100, 80);
    ///     let (action, response) = scroll.scroll_by_event(event, window_size, |source, _, coord| {
    ///         if source.is_primary() {
    ///             let icon = Some(kas::event::CursorIcon::Grabbing);
    ///             mgr.request_grab(id, source, coord, kas::event::GrabMode::Grab, icon);
    ///         }
    ///     });
    ///     *mgr |= action;
    ///     response.void_into()
    /// }
    /// ```
    ///
    /// If the returned [`TkAction`] is `None`, the scroll offset has not changed and
    /// the returned [`Response`] is either `None` or `Unhandled(..)`.
    /// If the returned [`TkAction`] is not `None`, the scroll offset has been
    /// updated and the second return value is `Response::None`.
    #[inline]
    pub fn scroll_by_event<PS: FnMut(PressSource, WidgetId, Coord)>(
        &mut self,
        event: Event,
        window_size: Size,
        mut on_press_start: PS,
    ) -> (TkAction, Response<VoidMsg>) {
        let mut action = TkAction::empty();
        let mut response = Response::None;

        match event {
            Event::Control(ControlKey::Home) => {
                action = self.set_offset(Offset::ZERO);
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
                    ControlKey::PageUp => PixelDelta(Offset(0, window_size.1 as i32 / 2)),
                    ControlKey::PageDown => PixelDelta(Offset(0, -(window_size.1 as i32 / 2))),
                    key => return (action, Response::Unhandled(Event::Control(key))),
                };

                let d = match delta {
                    LineDelta(x, y) => Offset(
                        (-self.scroll_rate * x) as i32,
                        (self.scroll_rate * y) as i32,
                    ),
                    PixelDelta(d) => d,
                };
                action = self.set_offset(self.offset - d);
            }
            Event::Scroll(delta) => {
                let d = match delta {
                    LineDelta(x, y) => Offset(
                        (-self.scroll_rate * x) as i32,
                        (self.scroll_rate * y) as i32,
                    ),
                    PixelDelta(d) => d,
                };
                action = self.set_offset(self.offset - d);
                if action.is_empty() {
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
#[widget(config=noauto)]
#[handler(send=noauto, msg = <W as event::Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[widget_core]
    core: CoreData,
    min_child_size: Size,
    scroll: ScrollComponent,
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

impl<W: Widget> ScrollWidget for ScrollRegion<W> {
    fn scroll_axes(&self, size: Size) -> (bool, bool) {
        (
            self.min_child_size.0 > size.0,
            self.min_child_size.1 > size.1,
        )
    }

    #[inline]
    fn max_scroll_offset(&self) -> Offset {
        self.scroll.max_offset()
    }

    #[inline]
    fn scroll_offset(&self) -> Offset {
        self.scroll.offset()
    }

    #[inline]
    fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
        *mgr |= self.scroll.set_offset(offset);
        self.scroll.offset()
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
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let child_size = rect.size.max(self.min_child_size);
        let child_rect = Rect::new(rect.pos, child_size);
        self.inner.set_rect(mgr, child_rect, align);
        let _ = self.scroll.set_sizes(rect.size, child_size);
    }

    #[inline]
    fn translation(&self, child_index: usize) -> Offset {
        match child_index {
            2 => self.scroll_offset(),
            _ => Offset::ZERO,
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }

        self.inner
            .find_id(coord + self.scroll_offset())
            .or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let disabled = disabled || self.is_disabled();
        draw_handle.clip_region(
            self.core.rect,
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

        let event = if id <= self.inner.id() {
            let event = self.scroll.offset_event(event);
            match self.inner.send(mgr, id, event) {
                Response::Unhandled(event) => event,
                Response::Focus(rect) => {
                    let (rect, action) = self.scroll.focus_rect(rect, self.core.rect);
                    *mgr |= action;
                    return Response::Focus(rect);
                }
                r => return r,
            }
        } else {
            debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
            event
        };

        let id = self.id();
        let (action, response) =
            self.scroll
                .scroll_by_event(event, self.core.rect.size, |source, _, coord| {
                    if source.is_primary() {
                        let icon = Some(event::CursorIcon::Grabbing);
                        mgr.request_grab(id, source, coord, event::GrabMode::Grab, icon);
                    }
                });
        if !action.is_empty() {
            *mgr |= action;
            Response::Focus(self.core.rect)
        } else {
            response.void_into()
        }
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
