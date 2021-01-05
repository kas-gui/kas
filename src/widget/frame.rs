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
        let size = size_handle.frame();
        let margins = Margins::ZERO;
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), size + size, margins);

        let child_rules = self.child.size_rules(size_handle, axis);
        let m = child_rules.margins();

        if axis.is_horizontal() {
            self.m0.0 = size.0 + m.0 as u32;
            self.m1.0 = size.0 + m.1 as u32;
        } else {
            self.m0.1 = size.1 + m.0 as u32;
            self.m1.1 = size.1 + m.1 as u32;
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
        if !self.rect().contains(coord) {
            return None;
        }
        self.child.find_id(coord).or(Some(self.id()))
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.outer_frame(self.core_data().rect);
        let disabled = disabled || self.is_disabled();
        self.child.draw(draw_handle, mgr, disabled);
    }
}

impl<W: HasBool + Widget> HasBool for Frame<W> {
    fn get_bool(&self) -> bool {
        self.child.get_bool()
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.child.set_bool(state)
    }
}

impl<W: HasStr + Widget> HasStr for Frame<W> {
    fn get_str(&self) -> &str {
        self.child.get_str()
    }
}

impl<W: HasString + Widget> HasString for Frame<W> {
    fn set_string(&mut self, text: String) -> TkAction {
        self.child.set_string(text)
    }
}

// TODO: HasFormatted

impl<W: SetAccel + Widget> SetAccel for Frame<W> {
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
        self.child.set_accel_string(accel)
    }
}
