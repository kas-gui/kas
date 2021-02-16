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
#[handler(msg = <W as Handler>::Msg)]
#[derive(Clone, Debug, Default, Widget)]
pub struct Frame<W: Widget> {
    #[widget_core]
    core: CoreData,
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

        if axis.is_horizontal() {
            self.offset.0 = offset;
            self.size.0 = size;
        } else {
            self.offset.1 = offset;
            self.size.1 = size;
        }
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

impl<W: HasBool + Widget> HasBool for Frame<W> {
    fn get_bool(&self) -> bool {
        self.inner.get_bool()
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.inner.set_bool(state)
    }
}

impl<W: HasStr + Widget> HasStr for Frame<W> {
    fn get_str(&self) -> &str {
        self.inner.get_str()
    }
}

impl<W: HasString + Widget> HasString for Frame<W> {
    fn set_string(&mut self, text: String) -> TkAction {
        self.inner.set_string(text)
    }
}

// TODO: HasFormatted

impl<W: SetAccel + Widget> SetAccel for Frame<W> {
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
        self.inner.set_accel_string(accel)
    }
}

impl<W: Widget> std::ops::Deref for Frame<W> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget> std::ops::DerefMut for Frame<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
