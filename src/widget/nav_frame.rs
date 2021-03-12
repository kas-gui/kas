// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A "navigable" wrapper

use kas::{event, prelude::*};

/// Navigation Frame wrapper
///
/// This widget is a wrapper that can be used to make a static widget such as a
/// `Label` navigable with the keyboard.
#[derive(Clone, Debug, Default, Widget)]
#[handler(handle=noauto)]
#[widget(config(key_nav = true))]
pub struct NavFrame<W: Widget> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub inner: W,
    offset: Offset,
    size: Size,
}

impl<W: Widget> NavFrame<W> {
    /// Construct a frame
    #[inline]
    pub fn new(inner: W) -> Self {
        NavFrame {
            core: Default::default(),
            inner,
            offset: Offset::ZERO,
            size: Size::ZERO,
        }
    }
}

impl<W: Widget> Layout for NavFrame<W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_rules = size_handle.nav_frame(axis.is_vertical());
        let child_rules = self.inner.size_rules(size_handle, axis);
        let (rules, offset, size) = frame_rules.surround_inner(child_rules);
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
        let input_state = self.input_state(mgr, disabled);
        draw_handle.nav_frame(self.rect(), input_state);
        self.inner.draw(draw_handle, mgr, input_state.disabled);
    }
}

impl<W: Widget> event::Handler for NavFrame<W> {
    type Msg = <W as Handler>::Msg;

    fn handle(&mut self, _mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::Activate => Response::Select,
            _ => Response::Unhandled,
        }
    }
}

impl<W: HasBool + Widget> HasBool for NavFrame<W> {
    fn get_bool(&self) -> bool {
        self.inner.get_bool()
    }

    fn set_bool(&mut self, state: bool) -> TkAction {
        self.inner.set_bool(state)
    }
}

impl<W: HasStr + Widget> HasStr for NavFrame<W> {
    fn get_str(&self) -> &str {
        self.inner.get_str()
    }
}

impl<W: HasString + Widget> HasString for NavFrame<W> {
    fn set_string(&mut self, text: String) -> TkAction {
        self.inner.set_string(text)
    }
}

// TODO: HasFormatted

impl<W: SetAccel + Widget> SetAccel for NavFrame<W> {
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction {
        self.inner.set_accel_string(accel)
    }
}

impl<W: Widget> std::ops::Deref for NavFrame<W> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<W: Widget> std::ops::DerefMut for NavFrame<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
