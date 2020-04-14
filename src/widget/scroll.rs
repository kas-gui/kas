// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use super::ScrollBar;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Event, Manager, Response};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;

/// A scrollable region
///
/// This region supports scrolling via mouse wheel and drag.
/// Optionally, it can have scroll bars (see [`ScrollRegion::show_bars`] and
/// [`ScrollRegion::with_bars`]).
///
/// Scroll regions translate their contents by an `offset`, which has a
/// minimum value of [`Coord::ZERO`] and a maximum value of
/// [`ScrollRegion::max_offset`].
#[handler(action, msg = <W as event::Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[widget_core]
    core: CoreData,
    min_child_size: Size,
    inner_size: Size,
    max_offset: Coord,
    offset: Coord,
    scroll_rate: f32,
    bar_width: u32,
    auto_bars: bool,
    show_bars: (bool, bool),
    #[widget]
    horiz_bar: ScrollBar<kas::Right>,
    #[widget]
    vert_bar: ScrollBar<kas::Down>,
    #[widget]
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
            bar_width: 0,
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
    /// Returns [`TkAction::None`] if the offset is identical to the old offset,
    /// or a greater action if not identical.
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
}

impl<W: Widget> Layout for ScrollRegion<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut rules = self.child.size_rules(size_handle, axis);
        if axis.is_horizontal() {
            self.min_child_size.0 = rules.min_size();
        } else {
            self.min_child_size.1 = rules.min_size();
        }
        let line_height = size_handle.line_height(TextClass::Label);
        self.scroll_rate = 3.0 * line_height as f32;
        rules.reduce_min_to(line_height);
        self.bar_width = (size_handle.scrollbar().0).1;

        if axis.is_horizontal() && (self.auto_bars || self.show_bars.1) {
            rules.append(self.vert_bar.size_rules(size_handle, axis));
        } else if axis.is_vertical() && (self.auto_bars || self.show_bars.0) {
            rules.append(self.horiz_bar.size_rules(size_handle, axis));
        }
        rules
    }

    fn set_rect(&mut self, rect: Rect, _: AlignHints) {
        self.core.rect = rect;
        // We use simplified layout code here
        let pos = rect.pos;
        self.inner_size = rect.size;

        if self.auto_bars {
            self.show_bars = (
                self.min_child_size.0 + self.bar_width > rect.size.0,
                self.min_child_size.1 + self.bar_width > rect.size.1,
            );
        }
        if self.show_bars.0 {
            self.inner_size.1 -= self.bar_width;
        }
        if self.show_bars.1 {
            self.inner_size.0 -= self.bar_width;
        }

        let child_size = self.inner_size.max(self.min_child_size);
        let child_rect = Rect::new(pos, child_size);
        self.child.set_rect(child_rect, AlignHints::NONE);
        self.max_offset = Coord::from(child_size) - Coord::from(self.inner_size);
        self.offset = self.offset.clamp(Coord::ZERO, self.max_offset);

        if self.show_bars.0 {
            let pos = Coord(pos.0, pos.1 + self.inner_size.1 as i32);
            let size = Size(self.inner_size.0, self.bar_width);
            self.horiz_bar
                .set_rect(Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .horiz_bar
                .set_limits(self.max_offset.0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(pos.0 + self.inner_size.0 as i32, pos.1);
            let size = Size(self.bar_width, self.core.rect.size.1);
            self.vert_bar.set_rect(Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .vert_bar
                .set_limits(self.max_offset.1 as u32, rect.size.1);
        }
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if self.horiz_bar.rect().contains(coord) {
            self.horiz_bar.find_id(coord)
        } else if self.vert_bar.rect().contains(coord) {
            self.vert_bar.find_id(coord)
        } else {
            self.child.find_id(coord + self.offset)
        }
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
            size: self.inner_size,
        };
        draw_handle.clip_region(rect, self.offset, &mut |handle| {
            self.child.draw(handle, mgr, disabled)
        });
    }
}

impl<W: Widget> event::EventHandler for ScrollRegion<W> {
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        let unhandled = |w: &mut Self, mgr: &mut Manager, event| match event {
            Event::Scroll(delta) => {
                let d = match delta {
                    event::ScrollDelta::LineDelta(x, y) => {
                        Coord((-w.scroll_rate * x) as i32, (w.scroll_rate * y) as i32)
                    }
                    event::ScrollDelta::PixelDelta(d) => d,
                };
                let action = w.set_offset(w.offset - d);
                if action != TkAction::None {
                    *mgr += action
                        + w.horiz_bar.set_value(w.offset.0 as u32)
                        + w.vert_bar.set_value(w.offset.1 as u32);
                    Response::None
                } else {
                    Response::Unhandled(Event::Scroll(delta))
                }
            }
            Event::PressStart { source, coord } if source.is_primary() => {
                mgr.request_grab(
                    w.id(),
                    source,
                    coord,
                    event::GrabMode::Grab,
                    Some(event::CursorIcon::Grabbing),
                );
                Response::None
            }
            e @ _ => Response::Unhandled(e),
        };

        if id <= self.horiz_bar.id() {
            return match Response::<Self::Msg>::try_from(self.horiz_bar.event(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => unhandled(self, mgr, event),
                Ok(r) => r,
                Err(msg) => {
                    *mgr += self.set_offset(Coord(msg as i32, self.offset.1));
                    Response::None
                }
            };
        } else if id <= self.vert_bar.id() {
            return match Response::<Self::Msg>::try_from(self.vert_bar.event(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => unhandled(self, mgr, event),
                Ok(r) => r,
                Err(msg) => {
                    *mgr += self.set_offset(Coord(self.offset.0, msg as i32));
                    Response::None
                }
            };
        } else if id == self.id() {
            return match event {
                Event::PressMove { delta, .. } => {
                    let action = self.set_offset(self.offset - delta);
                    if action != TkAction::None {
                        *mgr += action
                            + self.horiz_bar.set_value(self.offset.0 as u32)
                            + self.vert_bar.set_value(self.offset.1 as u32);
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
            event => event,
        };

        match self.child.event(mgr, id, event) {
            Response::None => Response::None,
            Response::Unhandled(event) => unhandled(self, mgr, event),
            e @ _ => e,
        }
    }
}
