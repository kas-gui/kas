// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `ScrollBar` control

use std::fmt::Debug;

use super::ScrollRegion;
use crate::event::{self, Address, Event, Handler, Manager, PressSource, Response};
use crate::geom::{Coord, Rect, Size};
use crate::layout::{AxisInfo, Direction, Horizontal, SizeRules, Vertical};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle};
use crate::{CoreData, TkWindow, Widget, WidgetCore};

/// A scroll bar
///
/// Scroll bars allow user-input of a value between 0 and a defined maximum,
/// and allow the size of the handle to be specified.
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBar<D: Direction> {
    #[core]
    core: CoreData,
    direction: D,
    // Terminology assumes vertical orientation:
    width: u32,
    min_handle_len: u32,
    handle_len: u32,
    handle_value: u32, // contract: > 0
    max_value: u32,
    value: u32,
    press_source: Option<PressSource>,
    press_offset: i32,
}

impl<D: Direction + Default> ScrollBar<D> {
    /// Construct a scroll bar
    ///
    /// Default values are assumed for all parameters.
    pub fn new() -> Self {
        ScrollBar::new_with_direction(D::default())
    }
}

impl<D: Direction> ScrollBar<D> {
    /// Construct a scroll bar with the given direction
    ///
    /// Default values are assumed for all parameters.
    #[inline]
    pub fn new_with_direction(direction: D) -> Self {
        ScrollBar {
            core: Default::default(),
            direction,
            width: 0,
            min_handle_len: 0,
            handle_len: 0,
            handle_value: 1,
            max_value: 0,
            value: 0,
            press_source: None,
            press_offset: 0,
        }
    }

    /// Set the page length
    ///
    /// See [`ScrollBar::set_limits`].
    #[inline]
    pub fn with_limits(mut self, max_value: u32, handle_value: u32) -> Self {
        self.set_limits(max_value, handle_value);
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
    pub fn set_limits(&mut self, max_value: u32, handle_value: u32) {
        debug_assert!(handle_value > 0);
        self.handle_value = handle_value.max(1);

        self.max_value = max_value;
        self.value = self.value.min(self.max_value);
        self.update_handle();
    }

    /// Get the current value
    #[inline]
    pub fn value(&self) -> u32 {
        self.value
    }

    /// Set the value
    pub fn set_value(&mut self, tk: &mut dyn TkWindow, value: u32) {
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            tk.redraw(self.id());
        }
    }

    #[inline]
    fn len(&self) -> u32 {
        match self.direction.is_vertical() {
            false => self.core.rect.size.0,
            true => self.core.rect.size.1,
        }
    }

    fn update_handle(&mut self) {
        let len = self.len();
        let total = self.max_value as u64 + self.handle_value as u64;
        let handle_len = self.handle_value as u64 * len as u64 / total;
        self.handle_len = (handle_len as u32).max(self.min_handle_len).min(len);
        self.value = self.value.min(self.max_value);
    }

    // translate value to position in local coordinates
    fn position(&self) -> u32 {
        let len = self.len() - self.handle_len;
        let lhs = self.value as u64 * len as u64;
        let rhs = self.max_value as u64;
        if rhs == 0 {
            return 0;
        }
        let pos = ((lhs + (rhs / 2)) / rhs) as u32;
        pos.min(len)
    }

    // true if not equal to old value
    fn set_position(&mut self, tk: &mut dyn TkWindow, position: u32) -> bool {
        let len = self.len() - self.handle_len;
        let lhs = position as u64 * self.max_value as u64;
        let rhs = len as u64;
        if rhs == 0 {
            debug_assert_eq!(self.value, 0);
            return false;
        }
        let value = ((lhs + (rhs / 2)) / rhs) as u32;
        let value = value.min(self.max_value);
        if value != self.value {
            self.value = value;
            tk.redraw(self.id());
            return true;
        }
        false
    }
}

impl<D: Direction> Widget for ScrollBar<D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let (thickness, _, min_len) = size_handle.scrollbar();
        self.width = thickness;
        if self.direction.is_vertical() == axis.vertical() {
            SizeRules::fixed(min_len)
        } else {
            SizeRules::fixed(thickness)
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        let (thickness, min_handle_len, _) = size_handle.scrollbar();
        self.width = thickness;
        self.min_handle_len = min_handle_len;
        self.core.rect = rect;
        self.update_handle();
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &event::Manager) {
        let dir = self.direction.is_vertical();
        let hl = ev_mgr.highlight_state(self.id());
        draw_handle.scrollbar(self.core.rect, dir, self.handle_len, self.position(), hl);
    }
}

impl<D: Direction> Handler for ScrollBar<D> {
    type Msg = u32;

    fn handle(&mut self, tk: &mut dyn TkWindow, _: Address, event: Event) -> Response<Self::Msg> {
        match event {
            Event::PressStart { source, coord, .. } => {
                // Interacting with a scrollbar with multiple presses
                // does not make sense. Any other gets aborted.
                // TODO: only if request_press_grab succeeds
                self.press_source = Some(source);
                tk.update_data(&mut |data| data.request_press_grab(source, self, coord));

                // Event delivery implies coord is over the scrollbar.
                let (pointer, offset) = match self.direction.is_vertical() {
                    false => (coord.0, self.core.rect.pos.0),
                    true => (coord.1, self.core.rect.pos.1),
                };
                let position = self.position() as i32;
                let h_start = offset + position;

                if pointer >= h_start && pointer < h_start + self.handle_len as i32 {
                    // coord is on the scroll handle
                    self.press_offset = position - pointer;
                    Response::None
                } else {
                    // coord is not on the handle; we move the bar immediately
                    self.press_offset = -offset - (self.handle_len / 2) as i32;
                    let position = (pointer + self.press_offset).max(0) as u32;
                    let moved = self.set_position(tk, position);
                    debug_assert!(moved);
                    tk.redraw(self.id());
                    Response::Msg(self.value)
                }
            }
            Event::PressMove { source, coord, .. } if Some(source) == self.press_source => {
                let pointer = match self.direction.is_vertical() {
                    false => coord.0,
                    true => coord.1,
                };
                let position = (pointer + self.press_offset).max(0) as u32;
                if self.set_position(tk, position) {
                    tk.redraw(self.id());
                    Response::Msg(self.value)
                } else {
                    Response::None
                }
            }
            Event::PressEnd { source, .. } if Some(source) == self.press_source => {
                self.press_source = None;
                Response::None
            }
            e @ _ => Manager::handle_generic(self, tk, e),
        }
    }
}

/// A scrollable region with scroll bars
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollBarRegion<W: Widget> {
    #[core]
    core: CoreData,
    #[widget]
    horiz_bar: ScrollBar<Horizontal>,
    #[widget]
    vert_bar: ScrollBar<Vertical>,
    #[widget]
    inner: ScrollRegion<W>,
    show: (bool, bool),
}

impl<W: Widget> ScrollBarRegion<W> {
    /// Construct a new scroll bar region around a child widget
    ///
    /// By default, a vertical scroll-bar is shown but not a horizontal bar.
    #[inline]
    pub fn new(child: W) -> Self {
        ScrollBarRegion {
            core: Default::default(),
            horiz_bar: ScrollBar::new(),
            vert_bar: ScrollBar::new(),
            inner: ScrollRegion::new(child),
            show: (false, true),
        }
    }

    /// Set which scroll bars are visible
    #[inline]
    pub fn with_bars(mut self, horiz: bool, vert: bool) -> Self {
        self.show = (horiz, vert);
        self
    }

    /// Set which scroll bars are visible
    #[inline]
    pub fn set_bars(&mut self, horiz: bool, vert: bool) {
        self.show = (horiz, vert);
    }

    /// Access inner widget directly
    #[inline]
    pub fn inner(&self) -> &W {
        &self.inner.inner()
    }

    /// Access inner widget directly
    #[inline]
    pub fn inner_mut(&mut self) -> &mut W {
        self.inner.inner_mut()
    }
}

impl<W: Widget> Widget for ScrollBarRegion<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let rules = self.inner.size_rules(size_handle, axis);
        if !axis.vertical() && self.show.1 {
            rules + self.vert_bar.size_rules(size_handle, axis)
        } else if axis.vertical() && self.show.0 {
            rules + self.horiz_bar.size_rules(size_handle, axis)
        } else {
            rules
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, mut rect: Rect) {
        // We use simplified layout code here
        self.core.rect = rect;
        if self.show.0 {
            rect.size.1 -= self.horiz_bar.width;
        }
        if self.show.1 {
            rect.size.0 -= self.vert_bar.width;
        }
        self.inner.set_rect(size_handle, rect);
        let inner_size = rect.size;
        let max_offset = self.inner.max_offset();

        if self.show.0 {
            let pos = Coord(rect.pos.0, rect.pos.1 + rect.size.1 as i32);
            let size = Size(rect.size.0, self.horiz_bar.width);
            self.horiz_bar.set_rect(size_handle, Rect { pos, size });
            self.horiz_bar.set_limits(max_offset.0 as u32, inner_size.0);
        }
        if self.show.1 {
            let pos = Coord(rect.pos.0 + rect.size.0 as i32, rect.pos.1);
            let size = Size(self.vert_bar.width, rect.size.1);
            self.vert_bar.set_rect(size_handle, Rect { pos, size });
            self.vert_bar.set_limits(max_offset.1 as u32, inner_size.1);
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &Manager) {
        if self.show.0 {
            self.horiz_bar.draw(draw_handle, ev_mgr);
        }
        if self.show.1 {
            self.vert_bar.draw(draw_handle, ev_mgr);
        }
        self.inner.draw(draw_handle, ev_mgr);
    }
}

impl<W: Widget + Handler> Handler for ScrollBarRegion<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(
        &mut self,
        tk: &mut dyn TkWindow,
        addr: Address,
        event: Event,
    ) -> Response<Self::Msg> {
        let do_horiz = |w: &mut Self, tk: &mut dyn TkWindow, addr, event| {
            match Response::<Self::Msg>::try_from(w.horiz_bar.handle(tk, addr, event)) {
                Ok(Response::Unhandled(Event::Action(action))) => {
                    let old_offset = w.inner.offset().0;
                    let r = w.inner.unhandled_action(tk, action).into();
                    let offset = w.inner.offset().0;
                    if old_offset != offset {
                        w.horiz_bar.set_value(tk, offset as u32);
                    }
                    r
                }
                Ok(r) => r,
                Err(msg) => {
                    let mut offset = w.inner.offset();
                    offset.0 = msg as i32;
                    w.inner.set_offset(tk, offset);
                    Response::None
                }
            }
        };
        let do_vert = |w: &mut Self, tk: &mut dyn TkWindow, addr, event| {
            match Response::<Self::Msg>::try_from(w.vert_bar.handle(tk, addr, event)) {
                Ok(Response::Unhandled(Event::Action(action))) => {
                    let old_offset = w.inner.offset().1;
                    let r = w.inner.unhandled_action(tk, action).into();
                    let offset = w.inner.offset().1;
                    if old_offset != offset {
                        w.vert_bar.set_value(tk, offset as u32);
                    }
                    r
                }
                Ok(r) => r,
                Err(msg) => {
                    let mut offset = w.inner.offset();
                    offset.1 = msg as i32;
                    w.inner.set_offset(tk, offset);
                    Response::None
                }
            }
        };
        let do_inner = |w: &mut Self, tk: &mut dyn TkWindow, addr, event| {
            let old_offset = w.inner.offset();
            let r = w.inner.handle(tk, addr, event);
            let offset = w.inner.offset();
            if old_offset != offset {
                // Inner scroll region moved; update scroll bars
                if old_offset.0 != offset.0 {
                    w.horiz_bar.set_value(tk, offset.0 as u32);
                }
                if old_offset.1 != offset.1 {
                    w.vert_bar.set_value(tk, offset.1 as u32);
                }
            }
            r
        };

        match addr {
            Address::Id(id) if id <= self.horiz_bar.id() => do_horiz(self, tk, addr, event),
            Address::Id(id) if id <= self.vert_bar.id() => do_vert(self, tk, addr, event),
            Address::Id(mut id) => {
                if id == self.id() {
                    // Forward any events to self to the inner region
                    id = self.inner.id();
                }
                if id <= self.inner.id() {
                    do_inner(self, tk, Address::Id(id), event)
                } else {
                    debug_assert!(false);
                    Response::Unhandled(event)
                }
            }
            Address::Coord(coord) if self.inner.rect().contains(coord) => {
                do_inner(self, tk, addr, event)
            }
            Address::Coord(coord) if self.horiz_bar.rect().contains(coord) => {
                do_horiz(self, tk, addr, event).into()
            }
            Address::Coord(coord) if self.vert_bar.rect().contains(coord) => {
                do_vert(self, tk, addr, event)
            }
            Address::Coord(_) => Response::Unhandled(event),
        }
    }
}
