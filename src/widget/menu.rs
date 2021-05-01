// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menus

use std::ops::{Deref, DerefMut};

mod menu_entry;
mod menubar;
mod submenu;

pub use menu_entry::{MenuEntry, MenuToggle};
pub use menubar::MenuBar;
pub use submenu::SubMenu;

use kas::{event, prelude::*};

/// Trait governing menus, sub-menus and menu-entries
pub trait Menu: Widget {
    /// Report whether one's own menu is open
    ///
    /// By default, this is `false`.
    fn menu_is_open(&self) -> bool {
        false
    }

    /// Open or close a sub-menu, including parents
    ///
    /// Given `Some(id) = target`, the sub-menu with this `id` should open its
    /// menu; if it has child-menus, these should close; and if any ancestors
    /// are menus, these should open.
    ///
    /// `target == None` implies that all menus should close.
    fn menu_path(&mut self, _mgr: &mut Manager, _target: Option<WidgetId>) {}
}

impl<M: 'static> WidgetCore for Box<dyn Menu<Msg = M>> {
    fn as_any(&self) -> &dyn std::any::Any {
        self.as_ref().as_any()
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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

impl<M: 'static> WidgetChildren for Box<dyn Menu<Msg = M>> {
    fn first_id(&self) -> WidgetId {
        self.as_ref().first_id()
    }
    fn record_first_id(&mut self, id: WidgetId) {
        self.as_mut().record_first_id(id)
    }
    fn num_children(&self) -> usize {
        self.as_ref().num_children()
    }
    fn get_child(&self, index: usize) -> Option<&dyn WidgetConfig> {
        self.as_ref().get_child(index)
    }
    fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn WidgetConfig> {
        self.as_mut().get_child_mut(index)
    }

    fn find_child(&self, id: WidgetId) -> Option<usize> {
        self.as_ref().find_child(id)
    }
    fn find_leaf(&self, id: WidgetId) -> Option<&dyn WidgetConfig> {
        self.as_ref().find_leaf(id)
    }
    fn find_leaf_mut(&mut self, id: WidgetId) -> Option<&mut dyn WidgetConfig> {
        self.as_mut().find_leaf_mut(id)
    }

    fn walk_children_dyn(&self, f: &mut dyn FnMut(&dyn WidgetConfig)) {
        self.as_ref().walk_children_dyn(f);
    }
    fn walk_children_mut_dyn(&mut self, f: &mut dyn FnMut(&mut dyn WidgetConfig)) {
        self.as_mut().walk_children_mut_dyn(f);
    }
}

impl<M: 'static> WidgetConfig for Box<dyn Menu<Msg = M>> {
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

impl<M: 'static> Layout for Box<dyn Menu<Msg = M>> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.as_mut().size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        self.as_mut().set_rect(mgr, rect, align);
    }

    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        self.as_ref().find_id(coord)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        self.as_ref().draw(draw_handle, mgr, disabled);
    }
}

impl<M: 'static> event::Handler for Box<dyn Menu<Msg = M>> {
    type Msg = M;

    fn activation_via_press(&self) -> bool {
        self.as_ref().activation_via_press()
    }

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        self.as_mut().handle(mgr, event)
    }
}

impl<M: 'static> event::SendEvent for Box<dyn Menu<Msg = M>> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        self.as_mut().send(mgr, id, event)
    }
}

impl<M: 'static> Widget for Box<dyn Menu<Msg = M>> {}

impl<M: 'static> Menu for Box<dyn Menu<Msg = M>> {
    fn menu_is_open(&self) -> bool {
        self.deref().menu_is_open()
    }
    fn menu_path(&mut self, mgr: &mut Manager, target: Option<WidgetId>) {
        self.deref_mut().menu_path(mgr, target)
    }
}

impl<M: Menu + Sized> Boxed<dyn Menu<Msg = M::Msg>> for M {
    fn boxed(self) -> Box<dyn Menu<Msg = M::Msg>> {
        Box::new(self)
    }
}
