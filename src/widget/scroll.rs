// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use super::ScrollBar;
use crate::event::{Action, Event, Handler, Manager, Response, ScrollDelta};
use crate::geom::{Coord, Rect, Size};
use crate::layout::{AxisInfo, Horizontal, SizeRules, Vertical};
use crate::theme::{DrawHandle, SizeHandle, TextClass};
use crate::{CoreData, Layout, TkAction, Widget, WidgetCore, WidgetId};

/// A scrollable region
///
/// This region supports scrolling via mouse wheel and drag.
/// Optionally, it can have scroll bars (see [`ScrollRegion::show_bars`] and
/// [`ScrollRegion::with_bars`]).
///
/// Scroll regions translate their contents by an `offset`, which has a
/// minimum value of [`Coord::ZERO`] and a maximum value of
/// [`ScrollRegion::max_offset`].
#[derive(Clone, Debug, Default)]
pub struct ScrollRegion<W: Widget> {
    core: CoreData,
    min_child_size: Size,
    inner_size: Size,
    max_offset: Coord,
    offset: Coord,
    scroll_rate: f32,
    auto_bars: bool,
    show_bars: (bool, bool),
    horiz_bar: ScrollBar<Horizontal>,
    vert_bar: ScrollBar<Vertical>,
    child: W,
}

impl<W: Widget> ScrollRegion<W> {
    /// Construct a new scroll region around a child widget
    #[inline]
    pub fn new(child: W) -> Self {
        ScrollRegion {
            core: Default::default(),
            min_child_size: Size::ZERO,
            inner_size: Size::ZERO,
            max_offset: Coord::ZERO,
            offset: Coord::ZERO,
            scroll_rate: 30.0,
            auto_bars: false,
            show_bars: (false, false),
            horiz_bar: ScrollBar::new(),
            vert_bar: ScrollBar::new(),
            child,
        }
    }

    /// Auto-enable bars
    ///
    /// If enabled, this automatically enables/disables scroll bars when
    /// resized.
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
    pub fn set_offset(&mut self, mgr: &mut Manager, offset: Coord) -> bool {
        let offset = offset.max(Coord::ZERO).min(self.max_offset);
        if offset != self.offset {
            self.offset = offset;
            mgr.send_action(TkAction::RegionMoved);
            return true;
        }
        false
    }
}

// TODO: we should use the derive implementation, but find_coord_mut needs a
// manual offset! Can we find a less tedious workaround?
impl<W: Widget> WidgetCore for ScrollRegion<W> {
    #[inline]
    fn core_data(&self) -> &CoreData {
        &self.core
    }
    #[inline]
    fn core_data_mut(&mut self) -> &mut CoreData {
        &mut self.core
    }

    #[inline]
    fn widget_name(&self) -> &'static str {
        "ScrollRegion"
    }

    #[inline]
    fn as_widget(&self) -> &dyn Widget {
        self
    }
    #[inline]
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }

    #[inline]
    fn len(&self) -> usize {
        3
    }
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn Widget> {
        match index {
            0 => Some(&self.horiz_bar),
            1 => Some(&self.vert_bar),
            2 => Some(&self.child),
            _ => None,
        }
    }
    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
        match index {
            0 => Some(&mut self.horiz_bar),
            1 => Some(&mut self.vert_bar),
            2 => Some(&mut self.child),
            _ => None,
        }
    }

    fn walk(&self, f: &mut dyn FnMut(&dyn Widget)) {
        self.horiz_bar.walk(f);
        self.vert_bar.walk(f);
        self.child.walk(f);
        f(self)
    }
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        self.horiz_bar.walk_mut(f);
        self.vert_bar.walk_mut(f);
        self.child.walk_mut(f);
        f(self)
    }

    fn find_coord_mut(&mut self, coord: Coord) -> Option<&mut dyn Widget> {
        if self.horiz_bar.rect().contains(coord) {
            self.horiz_bar.find_coord_mut(coord)
        } else if self.vert_bar.rect().contains(coord) {
            self.vert_bar.find_coord_mut(coord)
        } else {
            self.child.find_coord_mut(coord + self.offset)
        }
    }
}

impl<W: Widget> Widget for ScrollRegion<W> {}

impl<W: Widget> Layout for ScrollRegion<W> {
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

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        self.core.rect = rect;
        // We use simplified layout code here
        let pos = rect.pos;
        let mut size = rect.size;
        if self.auto_bars {
            self.show_bars = (
                self.min_child_size.0 > size.0,
                self.min_child_size.1 > size.1,
            );
        }

        if self.show_bars.0 {
            size.1 -= self.horiz_bar.width();
        }
        if self.show_bars.1 {
            size.0 -= self.vert_bar.width();
        }
        self.inner_size = size;

        let child_size = size.max(self.min_child_size);
        self.child.set_rect(
            size_handle,
            Rect {
                pos,
                size: child_size,
            },
        );
        self.max_offset = Coord::from(child_size) - Coord::from(rect.size);
        self.offset = self.offset.max(Coord::ZERO).min(self.max_offset);

        if self.show_bars.0 {
            let pos = Coord(pos.0, pos.1 + size.1 as i32);
            let size = Size(size.0, self.horiz_bar.width());
            self.horiz_bar.set_rect(size_handle, Rect { pos, size });
            self.horiz_bar
                .set_limits(self.max_offset.0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(pos.0 + size.0 as i32, pos.1);
            let size = Size(self.vert_bar.width(), self.core.rect.size.1);
            self.vert_bar.set_rect(size_handle, Rect { pos, size });
            self.vert_bar
                .set_limits(self.max_offset.1 as u32, rect.size.1);
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &Manager) {
        if self.show_bars.0 {
            self.horiz_bar.draw(draw_handle, mgr);
        }
        if self.show_bars.1 {
            self.vert_bar.draw(draw_handle, mgr);
        }
        let rect = Rect {
            pos: self.core.rect.pos,
            size: self.inner_size,
        };
        draw_handle.clip_region(rect, self.offset, &mut |handle| {
            self.child.draw(handle, mgr)
        });
    }
}

impl<W: Widget + Handler> Handler for ScrollRegion<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        let unhandled = |w: &mut Self, mgr: &mut Manager, event| match event {
            Event::Action(Action::Scroll(delta)) => {
                let d = match delta {
                    ScrollDelta::LineDelta(x, y) => {
                        Coord((-w.scroll_rate * x) as i32, (w.scroll_rate * y) as i32)
                    }
                    ScrollDelta::PixelDelta(d) => d,
                };
                if w.set_offset(mgr, w.offset - d) {
                    w.horiz_bar.set_value(mgr, w.offset.0 as u32);
                    w.vert_bar.set_value(mgr, w.offset.1 as u32);
                    Response::None
                } else {
                    Response::unhandled_action(Action::Scroll(delta))
                }
            }
            Event::PressStart { source, coord } if source.is_primary() => {
                mgr.request_press_grab(source, w, coord);
                Response::None
            }
            e @ _ => Response::Unhandled(e),
        };

        if id <= self.horiz_bar.id() {
            return match Response::<Self::Msg>::try_from(self.horiz_bar.handle(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => unhandled(self, mgr, event),
                Ok(r) => r,
                Err(msg) => {
                    self.set_offset(mgr, Coord(msg as i32, self.offset.1));
                    Response::None
                }
            };
        } else if id <= self.vert_bar.id() {
            return match Response::<Self::Msg>::try_from(self.vert_bar.handle(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => unhandled(self, mgr, event),
                Ok(r) => r,
                Err(msg) => {
                    self.set_offset(mgr, Coord(self.offset.0, msg as i32));
                    Response::None
                }
            };
        } else if id == self.id() {
            return match event {
                Event::PressMove { delta, .. } => {
                    if self.set_offset(mgr, self.offset - delta) {
                        self.horiz_bar.set_value(mgr, self.offset.0 as u32);
                        self.vert_bar.set_value(mgr, self.offset.1 as u32);
                    }
                    Response::None
                }
                Event::PressEnd { .. } => {
                    // consume due to request
                    Response::None
                }
                e @ _ => Response::Unhandled(e),
            };
        }

        let event = match event {
            a @ Event::Action(_) => a,
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
                end_id,
                coord,
            } => Event::PressEnd {
                source,
                end_id,
                coord: coord + self.offset,
            },
        };

        match self.child.handle(mgr, id, event) {
            Response::None => Response::None,
            Response::Unhandled(event) => unhandled(self, mgr, event),
            e @ _ => e,
        }
    }
}
