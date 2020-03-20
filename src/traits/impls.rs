// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Trait impls

use super::*;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Action, Event, Manager, Response};
use crate::geom::{Coord, Rect};
use crate::layout::{AxisInfo, SizeRules};
use crate::{AlignHints, CoreData, WidgetId};

impl<M> WidgetCore for Box<dyn Layout<Msg = M>> {
    fn core_data(&self) -> &CoreData {
        self.as_ref().core_data()
    }
    fn core_data_mut(&mut self) -> &mut CoreData {
        self.as_mut().core_data_mut()
    }

    fn widget_name(&self) -> &'static str {
        self.as_ref().widget_name()
    }

    fn as_widget(&self) -> &dyn Widget {
        self.as_ref().as_widget()
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self.as_mut().as_widget_mut()
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }
    fn get(&self, index: usize) -> Option<&dyn Widget> {
        self.as_ref().get(index)
    }
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
        self.as_mut().get_mut(index)
    }

    fn walk(&self, f: &mut dyn FnMut(&dyn Widget)) {
        self.as_ref().walk(f);
    }
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        self.as_mut().walk_mut(f);
    }
}

impl<M> WidgetConfig for Box<dyn Layout<Msg = M>> {
    fn configure(&mut self, mgr: &mut Manager) {
        self.as_mut().configure(mgr);
    }

    fn key_nav(&self) -> bool {
        self.as_ref().key_nav()
    }
    fn cursor_icon(&self) -> event::CursorIcon {
        self.as_ref().cursor_icon()
    }
}

impl<M> Widget for Box<dyn Layout<Msg = M>> {}

impl<M> event::Handler for Box<dyn Layout<Msg = M>> {
    type Msg = M;

    fn activation_via_press(&self) -> bool {
        self.as_ref().activation_via_press()
    }

    fn action(&mut self, mgr: &mut Manager, action: Action) -> Response<Self::Msg> {
        self.as_mut().action(mgr, action)
    }
}

impl<M> Layout for Box<dyn Layout<Msg = M>> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.as_mut().size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.as_mut().set_rect(size_handle, rect, align);
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        self.as_ref().find_id(coord)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        self.as_ref().draw(draw_handle, mgr);
    }

    fn event(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        self.as_mut().event(mgr, id, event)
    }
}

impl<M: 'static> Clone for Box<dyn Layout<Msg = M>> {
    fn clone(&self) -> Self {
        #[cfg(feature = "nightly")]
        unsafe {
            use crate::CloneTo;
            let mut x = Box::new_uninit();
            self.clone_to(x.as_mut_ptr());
            x.assume_init()
        }

        // Run-time failure is not ideal — but we would hit compile-issues which
        // don't necessarily correspond to actual usage otherwise due to
        // `derive(Clone)` on any widget produced by `make_widget!`.
        #[cfg(not(feature = "nightly"))]
        panic!("Clone for Box<dyn Widget> only supported on nightly");
    }
}
