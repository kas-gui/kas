// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use super::ScrollBar;
use kas::draw::{ClipRegion, TextClass};
use kas::event::ScrollDelta::{LineDelta, PixelDelta};
use kas::event::{self, ControlKey};
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
#[widget(config=noauto)]
#[handler(send=noauto, msg = <W as event::Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[widget_core]
    core: CoreData,
    min_child_size: Size,
    inner_size: Size,
    max_offset: Coord,
    offset: Coord,
    scroll_rate: f32,
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
            inner_size: Size::ZERO,
            max_offset: Coord::ZERO,
            offset: Coord::ZERO,
            scroll_rate: 30.0,
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
        self.scroll_rate = 3.0 * line_height as f32;
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
        self.inner_size = rect.size;

        let bar_width = (size_handle.scrollbar().0).1;
        if self.auto_bars {
            self.show_bars = (
                self.min_child_size.0 + bar_width > rect.size.0,
                self.min_child_size.1 + bar_width > rect.size.1,
            );
        }
        if self.show_bars.0 {
            self.inner_size.1 -= bar_width;
        }
        if self.show_bars.1 {
            self.inner_size.0 -= bar_width;
        }

        let child_size = self.inner_size.max(self.min_child_size);
        let child_rect = Rect::new(pos, child_size);
        self.inner
            .set_rect(size_handle, child_rect, AlignHints::NONE);
        self.max_offset = Coord::from(child_size) - Coord::from(self.inner_size);
        self.offset = self.offset.clamp(Coord::ZERO, self.max_offset);

        if self.show_bars.0 {
            let pos = Coord(pos.0, pos.1 + self.inner_size.1 as i32);
            let size = Size(self.inner_size.0, bar_width);
            self.horiz_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .horiz_bar
                .set_limits(self.max_offset.0 as u32, rect.size.0);
        }
        if self.show_bars.1 {
            let pos = Coord(pos.0 + self.inner_size.0 as i32, pos.1);
            let size = Size(bar_width, self.core.rect.size.1);
            self.vert_bar
                .set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);
            let _ = self
                .vert_bar
                .set_limits(self.max_offset.1 as u32, rect.size.1);
        }
    }

    #[inline]
    fn translation(&self, child_index: usize) -> Coord {
        match child_index {
            2 => self.offset,
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
            .or_else(|| self.inner.find_id(coord + self.offset))
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
            size: self.inner_size,
        };
        draw_handle.clip_region(rect, self.offset, ClipRegion::Scroll, &mut |handle| {
            self.inner.draw(handle, mgr, disabled)
        });
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
                    *mgr += self.set_offset(Coord(msg as i32, self.offset.1));
                    return Response::None;
                }
            }
        } else if id <= self.vert_bar.id() {
            match Response::<Self::Msg>::try_from(self.vert_bar.send(mgr, id, event)) {
                Ok(Response::Unhandled(event)) => event,
                Ok(r) => return r,
                Err(msg) => {
                    *mgr += self.set_offset(Coord(self.offset.0, msg as i32));
                    return Response::None;
                }
            }
        } else if id <= self.inner.id() {
            let event = match event {
                Event::PressStart {
                    source,
                    start_id,
                    coord,
                } => Event::PressStart {
                    source,
                    start_id,
                    coord: coord + self.offset,
                },
                Event::PressMove {
                    source,
                    cur_id,
                    coord,
                    delta,
                } => Event::PressMove {
                    source,
                    cur_id,
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

            match self.inner.send(mgr, id, event) {
                Response::Unhandled(event) => event,
                Response::Focus(rect) => {
                    let mut offset = self.offset;
                    offset = offset.max(rect.pos_end() - self.core.rect.pos_end());
                    offset = offset.min(rect.pos - self.core.rect.pos);
                    *mgr += self.set_offset(offset);
                    return Response::Focus(rect - self.offset);
                }
                r => return r,
            }
        } else {
            event
        };

        let scroll = |w: &mut Self, mgr: &mut Manager, delta| {
            let d = match delta {
                LineDelta(x, y) => Coord((-w.scroll_rate * x) as i32, (w.scroll_rate * y) as i32),
                PixelDelta(d) => d,
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
        };

        match event {
            Event::Control(key) => {
                let delta = match key {
                    ControlKey::Left => LineDelta(-1.0, 0.0),
                    ControlKey::Right => LineDelta(1.0, 0.0),
                    ControlKey::Up => LineDelta(0.0, 1.0),
                    ControlKey::Down => LineDelta(0.0, -1.0),
                    ControlKey::Home | ControlKey::End => {
                        let action = self.set_offset(match key {
                            ControlKey::Home => Coord::ZERO,
                            _ => self.max_offset,
                        });
                        if action != TkAction::None {
                            *mgr += action
                                + self.horiz_bar.set_value(self.offset.0 as u32)
                                + self.vert_bar.set_value(self.offset.1 as u32);
                        }
                        return Response::None;
                    }
                    ControlKey::PageUp => PixelDelta(Coord(0, self.core.rect.size.1 as i32 / 2)),
                    ControlKey::PageDown => {
                        PixelDelta(Coord(0, -(self.core.rect.size.1 as i32 / 2)))
                    }
                    key => return Response::Unhandled(Event::Control(key)),
                };
                scroll(self, mgr, delta)
            }
            Event::Scroll(delta) => scroll(self, mgr, delta),
            Event::PressStart { source, coord, .. } if source.is_primary() => {
                mgr.request_grab(
                    self.id(),
                    source,
                    coord,
                    event::GrabMode::Grab,
                    Some(event::CursorIcon::Grabbing),
                );
                Response::None
            }
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
        }
    }
}
