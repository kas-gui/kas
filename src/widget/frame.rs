// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use std::fmt::Debug;

use crate::class::*;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Event, Handler, Manager, Response};
use crate::geom::{Coord, Rect, Size};
use crate::layout::{AxisInfo, Margins, SizeRules};
use crate::macros::Widget;
use crate::{AlignHints, CoreData, CowString, Layout, Widget, WidgetCore, WidgetId};

/// A frame around content
///
/// This widget provides a simple abstraction: drawing a frame around its
/// contents.
#[widget_config]
#[handler(action, msg = <W as Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct Frame<W: Widget> {
    #[widget_core]
    core: CoreData,
    #[widget]
    child: W,
    m0: Size,
    m1: Size,
}

impl<W: Widget> Frame<W> {
    /// Construct a frame
    #[inline]
    pub fn new(child: W) -> Self {
        Frame {
            core: Default::default(),
            child,
            m0: Size::ZERO,
            m1: Size::ZERO,
        }
    }
}

impl<W: Widget> Layout for Frame<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.outer_frame();
        let margins = Margins::ZERO;
        let frame_rules = SizeRules::extract_fixed(axis.dir(), sides.0 + sides.1, margins);

        let child_rules = self.child.size_rules(size_handle, axis);
        let m = child_rules.margins();

        if axis.is_horizontal() {
            self.m0.0 = (sides.0).0 + m.0 as u32;
            self.m1.0 = (sides.1).0 + m.1 as u32;
        } else {
            self.m0.1 = (sides.0).1 + m.0 as u32;
            self.m1.1 = (sides.1).1 + m.1 as u32;
        }

        child_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, mut rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        rect.pos += self.m0;
        rect.size -= self.m0 + self.m1;
        self.child.set_rect(size_handle, rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if let Some(id) = self.child.find_id(coord) {
            Some(id)
        } else {
            Some(self.id())
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        draw_handle.outer_frame(self.core_data().rect);
        self.child.draw(draw_handle, mgr);
    }
}

impl<W: Widget> event::EventHandler for Frame<W> {
    #[inline]
    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if id <= self.child.id() {
            self.child.event(mgr, id, event)
        } else {
            debug_assert!(id == self.id(), "Layout::event: bad WidgetId");
            Response::Unhandled(event)
        }
    }
}

impl<W: HasBool + Widget> HasBool for Frame<W> {
    fn get_bool(&self) -> bool {
        self.child.get_bool()
    }

    fn set_bool(&mut self, mgr: &mut Manager, state: bool) {
        self.child.set_bool(mgr, state);
    }
}

impl<W: HasText + Widget> HasText for Frame<W> {
    fn get_text(&self) -> &str {
        self.child.get_text()
    }

    fn set_cow_string(&mut self, mgr: &mut Manager, text: CowString) {
        self.child.set_cow_string(mgr, text);
    }
}

impl<W: Editable + Widget> Editable for Frame<W> {
    fn is_editable(&self) -> bool {
        self.child.is_editable()
    }

    fn set_editable(&mut self, editable: bool) {
        self.child.set_editable(editable);
    }
}
