// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use std::fmt::Debug;

use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Handler};
use crate::geom::{Coord, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{AlignHints, CoreData, Layout, Widget, WidgetCore, WidgetId};

/// A frame around content
///
/// This widget provides a simple abstraction: drawing a frame around its
/// contents.
#[widget]
#[handler(msg = <W as Handler>::Msg, generics = <> where W: Handler)]
#[derive(Clone, Debug, Default, Widget)]
pub struct Frame<W: Widget> {
    #[core]
    core: CoreData,
    #[widget]
    child: W,
}

impl<W: Widget> Frame<W> {
    /// Construct a frame
    #[inline]
    pub fn new(child: W) -> Self {
        Frame {
            core: Default::default(),
            child,
        }
    }
}

impl<W: Widget> Layout for Frame<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sizes = size_handle.outer_frame();
        self.child.size_rules(size_handle, axis)
            + axis.extract_size(sizes.0)
            + axis.extract_size(sizes.1)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, mut rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        let sizes = size_handle.outer_frame();
        rect.pos += sizes.0;
        rect.size -= sizes.0 + sizes.1;
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
