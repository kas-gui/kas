// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Scroll region

use std::fmt::Debug;

use crate::event::{Event, Handler, Manager};
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
        rules.reduce_min_to(size_handle.line_height(TextClass::Label));
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

    fn draw(
        &self,
        draw_handle: &mut dyn DrawHandle,
        mut offset: kas::geom::Coord,
        ev_mgr: &Manager,
    ) {
        offset = offset + self.offset;
        draw_handle.clip_to(self.core.rect, &mut |handle| {
            self.child.draw(handle, offset, ev_mgr)
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
            child,
        }
    }
}

impl<W: Widget + Handler> Handler for ScrollRegion<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(&mut self, tk: &mut dyn TkWindow, event: Event) -> Self::Msg {
        match event {
            event @ Event::ToChild(..) => self.child.handle(tk, event),
            Event::ToCoord(coord, event) => self
                .child
                .handle(tk, Event::ToCoord(coord - self.offset, event)),
        }
    }
}
