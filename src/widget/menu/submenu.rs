// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::MenuFrame;
use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Event, Manager, Response};
use kas::layout::{AxisInfo, Margins, SizeRules};
use kas::prelude::*;
use kas::widget::Column;
use kas::WindowId;

/// A sub-menu
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct SubMenu<D: Directional, W: Widget> {
    #[widget_core]
    core: CoreData,
    direction: D,
    label: CowString,
    label_off: Coord,
    #[widget]
    pub list: MenuFrame<Column<W>>,
    popup_id: Option<WindowId>,
}

impl<D: Directional + Default, W: Widget> SubMenu<D, W> {
    /// Construct a sub-menu
    #[inline]
    pub fn new<S: Into<CowString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new_with_direction(Default::default(), label, list)
    }
}

impl<W: Widget> SubMenu<kas::Right, W> {
    /// Construct a sub-menu, opening to the right
    // NOTE: this is used since we can't infer direction of a boxed SubMenu.
    // Consider only accepting an enum of special menu widgets?
    // Then we can pass type information.
    #[inline]
    pub fn right<S: Into<CowString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new(label, list)
    }
}

impl<W: Widget> SubMenu<kas::Down, W> {
    /// Construct a sub-menu, opening downwards
    #[inline]
    pub fn down<S: Into<CowString>>(label: S, list: Vec<W>) -> Self {
        SubMenu::new(label, list)
    }
}

impl<D: Directional, W: Widget> SubMenu<D, W> {
    /// Construct a sub-menu
    #[inline]
    pub fn new_with_direction<S: Into<CowString>>(direction: D, label: S, list: Vec<W>) -> Self {
        SubMenu {
            core: Default::default(),
            direction,
            label: label.into(),
            label_off: Coord::ZERO,
            list: MenuFrame::new(Column::new(list)),
            popup_id: None,
        }
    }

    pub(crate) fn menu_is_open(&self) -> bool {
        self.popup_id.is_some()
    }
    fn open_menu(&mut self, mgr: &mut Manager) {
        if self.popup_id.is_none() {
            let id = mgr.add_popup(kas::Popup {
                id: self.list.id(),
                parent: self.id(),
                direction: self.direction.as_direction(),
            });
            self.popup_id = Some(id);
        }
    }
    fn close_menu(&mut self, mgr: &mut Manager) {
        if let Some(id) = self.popup_id {
            mgr.close_window(id);
            self.popup_id = None;
        }
    }
}

impl<D: Directional, W: Widget> kas::Layout for SubMenu<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let size = size_handle.menu_frame();
        self.label_off = size.into();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), size + size, Margins::ZERO);
        let text_rules = size_handle.text_bound(&self.label, TextClass::Label, axis);
        text_rules.surrounded_by(frame_rules, true)
    }

    fn spatial_range(&self) -> (usize, usize) {
        // We have no child within our rect; return an empty range
        (0, std::usize::MAX)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.menu_entry(self.core.rect, self.input_state(mgr, disabled));
        let rect = Rect {
            pos: self.core.rect.pos + self.label_off,
            size: self.core.rect.size - self.label_off.into(),
        };
        let align = (Align::Begin, Align::Centre);
        draw_handle.text(rect, &self.label, TextClass::Label, align);
    }
}

impl<D: Directional, M, W: Widget<Msg = M>> event::Handler for SubMenu<D, W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate | Event::OpenPopup => {
                if self.popup_id.is_none() {
                    self.open_menu(mgr);
                }
            }
            Event::ClosePopup => {
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                    self.popup_id = None;
                }
            }
            event => return Response::Unhandled(event),
        }
        Response::None
    }
}

impl<D: Directional, W: Widget> event::SendEvent for SubMenu<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.list.id() {
            let r = self.list.send(mgr, id, event);
            if r.is_msg() {
                self.close_menu(mgr);
            }
            r
        } else {
            Manager::handle_generic(self, mgr, event)
        }
    }
}

impl<D: Directional, W: Widget> HasText for SubMenu<D, W> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_cow_string(&mut self, text: CowString) -> TkAction {
        self.label = text;
        TkAction::Redraw
    }
}
