// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use super::ScrollBar;
use crate::event::{Action, Address, Event, Handler, Manager, Response, ScrollDelta};
use crate::geom::{Coord, Rect, Size};
use crate::layout::{AxisInfo, Horizontal, SizeRules, Vertical};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle, TextClass};
use crate::{CoreData, TkWindow, Widget, WidgetCore};

/// A scrollable region
///
/// This region supports scrolling via mouse wheel and drag.
/// Optionally, it can have scroll bars (see [`ScrollRegion::show_bars`] and
/// [`ScrollRegion::with_bars`]).
///
/// Scroll regions translate their contents by an `offset`, which has a
/// minimum value of [`Coord::ZERO`] and a maximum value of
/// [`ScrollRegion::max_offset`].
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[core]
    core: CoreData,
    offset: Coord,
    min_child_size: Size,
    max_offset: Coord,
    scroll_rate: f32,
    show_bars: (bool, bool),
    #[widget]
    horiz_bar: ScrollBar<Horizontal>,
    #[widget]
    vert_bar: ScrollBar<Vertical>,
    #[widget]
    child: W,
}

impl<W: Widget> ScrollRegion<W> {
    /// Construct a new scroll region around a child widget
    #[inline]
    pub fn new(child: W) -> Self {
        ScrollRegion {
            core: Default::default(),
            offset: Coord::ZERO,
            max_offset: Coord::ZERO,
            min_child_size: Size::ZERO,
            scroll_rate: 30.0,
            show_bars: (false, false),
            horiz_bar: ScrollBar::new(),
            vert_bar: ScrollBar::new(),
            child,
        }
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
        &self.child
    }

    /// Access inner widget directly
    #[inline]
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.child
    }

    /// Get the maximum offset
    #[inline]
    pub fn max_offset(&self) -> Coord {
        self.max_offset
    }

    /// Get the current offset
    #[inline]
    pub fn offset(&self) -> Coord {
        self.offset
    }

    /// Set the scroll offset
    ///
    /// Returns true if the offset is not identical to the old offset.
    #[inline]
    pub fn set_offset(&mut self, tk: &mut dyn TkWindow, offset: Coord) -> bool {
        let offset = offset.max(Coord::ZERO).min(self.max_offset);
        if offset != self.offset {
            self.offset = offset;
            tk.redraw(self.id());
            return true;
        }
        false
    }
}

impl<W: Widget> Widget for ScrollRegion<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut rules = self.child.size_rules(size_handle, axis);
        if !axis.vertical() {
            self.min_child_size.0 = rules.min_size();
        } else {
            self.min_child_size.1 = rules.min_size();
        }
        let line_height = size_handle.line_height(TextClass::Label);
        self.scroll_rate = 3.0 * line_height as f32;
        rules.reduce_min_to(line_height);

        if !axis.vertical() && self.show_bars.1 {
            rules + self.vert_bar.size_rules(size_handle, axis)
        } else if axis.vertical() && self.show_bars.0 {
            rules + self.horiz_bar.size_rules(size_handle, axis)
        } else {
            rules
        }
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, mut rect: Rect) {
        // We use simplified layout code here
        self.core.rect = rect;
        if self.show_bars.0 {
            rect.size.1 -= self.horiz_bar.width();
        }
        if self.show_bars.1 {
            rect.size.0 -= self.vert_bar.width();
        }

        let pos = rect.pos;
        let size = rect.size.max(self.min_child_size);
        self.child.set_rect(size_handle, Rect { pos, size });
        self.max_offset = Coord::from(size) - Coord::from(rect.size);
        self.offset = self.offset.max(Coord::ZERO).min(self.max_offset);

        if self.show_bars.0 {
            let pos = Coord(rect.pos.0, rect.pos.1 + rect.size.1 as i32);
            let size = Size(rect.size.0, self.horiz_bar.width());
            self.horiz_bar.set_rect(size_handle, Rect { pos, size });
            self.horiz_bar
                .set_limits(self.max_offset.0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(rect.pos.0 + rect.size.0 as i32, rect.pos.1);
            let size = Size(self.vert_bar.width(), rect.size.1);
            self.vert_bar.set_rect(size_handle, Rect { pos, size });
            self.vert_bar
                .set_limits(self.max_offset.1 as u32, rect.size.1);
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &Manager) {
        if self.show_bars.0 {
            self.horiz_bar.draw(draw_handle, ev_mgr);
        }
        if self.show_bars.1 {
            self.vert_bar.draw(draw_handle, ev_mgr);
        }
        draw_handle.clip_region(self.core.rect, self.offset, &mut |handle| {
            self.child.draw(handle, ev_mgr)
        });
    }
}

impl<W: Widget + Handler> Handler for ScrollRegion<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(
        &mut self,
        tk: &mut dyn TkWindow,
        addr: Address,
        event: Event,
    ) -> Response<Self::Msg> {
        let unhandled_action = |w: &mut Self, tk: &mut dyn TkWindow, action| match action {
            Action::Scroll(delta) => {
                let d = match delta {
                    ScrollDelta::LineDelta(x, y) => {
                        Coord((-w.scroll_rate * x) as i32, (w.scroll_rate * y) as i32)
                    }
                    ScrollDelta::PixelDelta(d) => d,
                };
                if w.set_offset(tk, w.offset - d) {
                    w.horiz_bar.set_value(tk, w.offset.0 as u32);
                    w.vert_bar.set_value(tk, w.offset.1 as u32);
                    Response::None
                } else {
                    Response::unhandled_action(Action::Scroll(delta))
                }
            }
            a @ _ => Response::unhandled_action(a),
        };

        let do_horiz = |w: &mut Self, tk: &mut dyn TkWindow, addr, event| {
            match Response::<Self::Msg>::try_from(w.horiz_bar.handle(tk, addr, event)) {
                Ok(Response::Unhandled(Event::Action(action))) => unhandled_action(w, tk, action),
                Ok(r) => r,
                Err(msg) => {
                    w.set_offset(tk, Coord(msg as i32, w.offset.1));
                    Response::None
                }
            }
        };
        let do_vert = |w: &mut Self, tk: &mut dyn TkWindow, addr, event| {
            match Response::<Self::Msg>::try_from(w.vert_bar.handle(tk, addr, event)) {
                Ok(Response::Unhandled(Event::Action(action))) => unhandled_action(w, tk, action),
                Ok(r) => r,
                Err(msg) => {
                    w.set_offset(tk, Coord(w.offset.0, msg as i32));
                    Response::None
                }
            }
        };

        let addr = match addr {
            Address::Id(id) if id <= self.horiz_bar.id() => return do_horiz(self, tk, addr, event),
            Address::Id(id) if id <= self.vert_bar.id() => return do_vert(self, tk, addr, event),
            Address::Id(id) if id == self.id() => {
                let r = match event {
                    Event::PressMove { delta, .. } => {
                        if self.set_offset(tk, self.offset - delta) {
                            self.horiz_bar.set_value(tk, self.offset.0 as u32);
                            self.vert_bar.set_value(tk, self.offset.1 as u32);
                        }
                        Response::None
                    }
                    Event::PressEnd { .. } => {
                        // consume due to request
                        Response::None
                    }
                    e @ _ => Response::Unhandled(e),
                };
                return r;
            }
            a @ Address::Id(_) => a,
            Address::Coord(coord) if self.horiz_bar.rect().contains(coord) => {
                return do_horiz(self, tk, addr, event);
            }
            Address::Coord(coord) if self.vert_bar.rect().contains(coord) => {
                return do_vert(self, tk, addr, event);
            }
            Address::Coord(coord) => Address::Coord(coord + self.offset),
        };
        let event = match event {
            a @ Event::Action(_) | a @ Event::Identify => a,
            Event::PressStart { source, coord } => Event::PressStart {
                source,
                coord: coord + self.offset,
            },
            Event::PressMove {
                source,
                coord,
                delta,
            } => Event::PressMove {
                source,
                coord: coord + self.offset,
                delta,
            },
            Event::PressEnd {
                source,
                start_id,
                end_id,
                coord,
            } => Event::PressEnd {
                source,
                start_id,
                end_id,
                coord: coord + self.offset,
            },
        };

        match self.child.handle(tk, addr, event) {
            Response::None => Response::None,
            Response::Unhandled(Event::Action(action)) => unhandled_action(self, tk, action),
            Response::Unhandled(Event::PressStart { source, coord }) if source.is_primary() => {
                tk.update_data(&mut |data| data.request_press_grab(source, self, coord));
                Response::None
            }
            e @ _ => e,
        }
    }
}
