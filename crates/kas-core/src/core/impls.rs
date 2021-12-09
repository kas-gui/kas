// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Trait impls

use std::any::Any;

use super::*;
use crate::draw::{DrawHandle, SizeHandle};
use crate::event::{self, Event, Manager, ManagerState, Response};
use crate::geom::{Coord, Rect};
use crate::layout::{AlignHints, AxisInfo, SizeRules};
use crate::{CoreData, WidgetId};

impl<M: 'static> WidgetCore for Box<dyn Widget<Msg = M>> {
    fn as_any(&self) -> &dyn Any {
        self.as_ref().as_any()
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.as_mut().as_any_mut()
    }

    fn core_data(&self) -> &CoreData {
        self.as_ref().core_data()
    }
    fn core_data_mut(&mut self) -> &mut CoreData {
        self.as_mut().core_data_mut()
    }

    fn widget_name(&self) -> &'static str {
        self.as_ref().widget_name()
    }

    fn as_widget(&self) -> &dyn WidgetConfig {
        self.as_ref().as_widget()
    }
    fn as_widget_mut(&mut self) -> &mut dyn WidgetConfig {
        self.as_mut().as_widget_mut()
    }
}

impl<M: 'static> WidgetChildren for Box<dyn Widget<Msg = M>> {
    fn num_children(&self) -> usize {
        self.as_ref().num_children()
    }
    fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig> {
        self.as_ref().get_child(index)
    }
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.as_mut().get_child_mut(index)
    }

    fn find_child_index(&self, id: WidgetId) -> Option<usize> {
        self.as_ref().find_child_index(id)
    }
    fn find_leaf(&self, id: WidgetId) -> Option<&dyn WidgetConfig> {
        self.as_ref().find_leaf(id)
    }
    fn find_leaf_mut(&mut self, id: WidgetId) -> Option<&mut dyn WidgetConfig> {
        self.as_mut().find_leaf_mut(id)
    }
}

impl<M: 'static> WidgetConfig for Box<dyn Widget<Msg = M>> {
    fn configure(&mut self, mgr: &mut Manager) {
        self.as_mut().configure(mgr);
    }

    fn key_nav(&self) -> bool {
        self.as_ref().key_nav()
    }
    fn hover_highlight(&self) -> bool {
        self.as_ref().hover_highlight()
    }
    fn cursor_icon(&self) -> event::CursorIcon {
        self.as_ref().cursor_icon()
    }
}

impl<M: 'static> Layout for Box<dyn Widget<Msg = M>> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.as_mut().size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.as_mut().set_rect(mgr, rect, align);
    }

    fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
        self.as_mut().find_id(coord)
    }

    fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
        self.as_mut().draw(draw, mgr, disabled);
    }
}

impl<M: 'static> event::Handler for Box<dyn Widget<Msg = M>> {
    type Msg = M;

    fn activation_via_press(&self) -> bool {
        self.as_ref().activation_via_press()
    }

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        self.as_mut().handle(mgr, event)
    }
}

impl<M: 'static> event::SendEvent for Box<dyn Widget<Msg = M>> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        self.as_mut().send(mgr, id, event)
    }
}

impl<M: 'static> Widget for Box<dyn Widget<Msg = M>> {}
