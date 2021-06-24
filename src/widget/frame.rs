// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A simple frame

use kas::{event, prelude::*};

/// A frame around content
///
/// This widget provides a simple abstraction: drawing a frame around its
/// contents.
#[derive(Clone, Debug, Default, Widget)]
#[handler(msg = <W as Handler>::Msg)]
#[widget_derive(class_traits, Deref, DerefMut)]
pub struct Frame<W: Widget> {
    #[widget_core]
    core: CoreData,
    #[widget_derive]
    #[widget]
    pub inner: W,
    offset: Offset,
    size: Size,
}

impl<W: Widget> Frame<W> {
    /// Construct a frame
    #[inline]
    pub fn new(inner: W) -> Self {
        Frame {
            core: Default::default(),
            inner,
            offset: Offset::ZERO,
            size: Size::ZERO,
        }
    }
}

impl<W: Widget> Layout for Frame<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.frame(axis.is_vertical());
        let child_rules = self.inner.size_rules(size_handle, axis);
        let (rules, offset, size) = frame_rules.surround(child_rules);
        self.offset.set_component(axis, offset);
        self.size.set_component(axis, size);
        rules
    }

    fn set_rect(&mut self, mgr: &mut Manager, mut rect: Rect, align: AlignHints) {
        self.core.rect = rect;
        rect.pos += self.offset;
        rect.size -= self.size;
        self.inner.set_rect(mgr, rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        self.inner.find_id(coord).or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.outer_frame(self.core_data().rect);
        let disabled = disabled || self.is_disabled();
        self.inner.draw(draw_handle, mgr, disabled);
    }
}
