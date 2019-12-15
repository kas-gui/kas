// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use crate::event::{Action, Event, EventChild, Handler, Manager, Response, ScrollDelta};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle, TextClass};
use crate::{CoreData, TkWindow, Widget, WidgetCore};
use kas::geom::{Coord, Rect, Size};

/// A scrollable region
#[widget]
#[derive(Clone, Debug, Default, Widget)]
pub struct ScrollRegion<W: Widget> {
    #[core]
    core: CoreData,
    offset: Coord,
    min_child_size: Size,
    min_offset: Coord,
    scroll_rate: f32,
    #[widget]
    child: W,
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
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        self.core_data_mut().rect = rect;
        let size = rect.size.max(self.min_child_size);
        self.child.set_rect(
            size_handle,
            Rect {
                pos: rect.pos,
                size,
            },
        );
        self.min_offset = Coord::from(rect.size) - Coord::from(size);
        self.offset = self.offset.min(Coord::ZERO).max(self.min_offset);
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &Manager) {
        draw_handle.clip_region(self.core.rect, self.offset, &mut |handle| {
            self.child.draw(handle, ev_mgr)
        });
    }
}

impl<W: Widget> ScrollRegion<W> {
    /// Construct a new scroll region around a child widget
    pub fn new(child: W) -> Self {
        ScrollRegion {
            core: Default::default(),
            offset: Coord::ZERO,
            min_offset: Coord::ZERO,
            min_child_size: Size::ZERO,
            scroll_rate: 30.0,
            child,
        }
    }
}

impl<W: Widget + Handler> Handler for ScrollRegion<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(&mut self, tk: &mut dyn TkWindow, event: Event) -> Response<Self::Msg> {
        let translate_event = |event| match event {
            a @ EventChild::Action(_) | a @ EventChild::Identify => a,
            EventChild::PressStart { source, coord } => EventChild::PressStart {
                source,
                coord: coord - self.offset,
            },
            EventChild::PressMove {
                source,
                coord,
                delta,
            } => EventChild::PressMove {
                source,
                coord: coord - self.offset,
                delta,
            },
            EventChild::PressEnd {
                source,
                start_id,
                coord,
            } => EventChild::PressEnd {
                source,
                start_id,
                coord: coord - self.offset,
            },
        };
        let event = match event {
            Event::ToChild(id, e) => Event::ToChild(id, translate_event(e)),
            Event::ToCoord(coord, e) => Event::ToCoord(coord - self.offset, translate_event(e)),
        };

        match self.child.handle(tk, event) {
            Response::None => Response::None,
            Response::Unhandled(EventChild::Action(Action::Scroll(delta))) => {
                let d = match delta {
                    ScrollDelta::LineDelta(x, y) => Coord(
                        (-self.scroll_rate * x) as i32,
                        (self.scroll_rate * y) as i32,
                    ),
                    ScrollDelta::PixelDelta(d) => d,
                };
                let offset = (self.offset + d).min(Coord::ZERO).max(self.min_offset);
                if offset != self.offset {
                    self.offset = offset;
                    tk.redraw(self.id());
                    Response::None
                } else {
                    Response::Unhandled(EventChild::Action(Action::Scroll(delta)))
                }
            }
            e @ _ => e,
        }
    }
}
