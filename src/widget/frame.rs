// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use std::fmt::Debug;

use crate::class::*;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Handler, Manager};
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

    fn set_text<T: ToString>(&mut self, mgr: &mut Manager, text: T)
    where
        Self: Sized,
    {
        self.child.set_text(mgr, text);
    }

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
        self.child.set_string(mgr, text);
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
