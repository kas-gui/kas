// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Event, Manager, Response};
use kas::layout::{AxisInfo, SizeRules};
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
    #[widget]
    pub list: Column<W>,
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
            list: Column::new(list),
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
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), sides.0 + sides.1, margins);

        let content_rules = size_handle.text_bound(&self.label, TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, _align: kas::AlignHints) {
        self.core.rect = rect;
    }

    fn spatial_range(&self) -> (usize, usize) {
        // We have no child within our rect; return an empty range
        (0, std::usize::MAX)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let mut state = self.input_state(mgr, disabled);
        if self.popup_id.is_some() {
            state.depress = true;
        }
        draw_handle.button(self.core.rect, state);
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.core.rect, &self.label, TextClass::Button, align);
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
