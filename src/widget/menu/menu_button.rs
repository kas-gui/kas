// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menu Button

use kas::class::HasText;
use kas::draw::{DrawHandle, SizeHandle, TextClass};
use kas::event::{Event, GrabMode, Manager, Response};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;
use kas::WindowId;

/// A pop-up menu
///
/// This widget opens another widget as a pop-up when clicked. It also supports
/// drag interactions which send [`Event::Activate`] to the pop-up widget under
/// the mouse on click-release.
///
/// Messages from the pop-up widget are propegated to this widget's parent when
/// emitted, and the menu is closed when this happens. Because of this it is
/// important that interactive widgets do emit a message when activated.
#[widget(config(key_nav = true))]
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct MenuButton<W: Widget> {
    #[widget_core]
    core: CoreData,
    label: CowString,
    #[widget]
    popup: W,
    opening: bool,
    popup_id: Option<WindowId>,
}

impl<W: Widget> MenuButton<W> {
    /// Construct a pop-up menu
    #[inline]
    pub fn new<S: Into<CowString>>(label: S, popup: W) -> Self {
        MenuButton {
            core: Default::default(),
            label: label.into(),
            popup,
            opening: false,
            popup_id: None,
        }
    }

    fn open_menu(&mut self, mgr: &mut Manager) {
        if self.popup_id.is_none() {
            let id = mgr.add_popup(kas::Popup {
                id: self.popup.id(),
                parent: self.id(),
                direction: Direction::Down,
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

impl<W: Widget> kas::Layout for MenuButton<W> {
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

impl<M, W: Widget<Msg = M>> event::Handler for MenuButton<W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => {
                if self.popup_id.is_none() {
                    self.open_menu(mgr);
                } else {
                    self.close_menu(mgr);
                }
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => {
                if self.is_ancestor_of(start_id) {
                    if source.is_primary() {
                        mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                        mgr.set_grab_depress(source, Some(start_id));
                        self.opening = self.popup_id.is_none();
                    }
                } else {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                        self.popup_id = None;
                    }
                    return Response::Unhandled(Event::None);
                }
            }
            Event::PressMove { source, cur_id, .. } => {
                if cur_id == Some(self.id()) {
                    self.open_menu(mgr);
                    mgr.set_grab_depress(source, cur_id);
                }
            }
            Event::PressEnd { end_id, coord, .. } => {
                if self.rect().contains(coord) {
                    if end_id == Some(self.id()) && self.opening {
                        self.open_menu(mgr);
                    } else {
                        self.close_menu(mgr);
                    }
                } else if self.popup_id.is_some() && self.popup.rect().contains(coord) {
                    if let Some(id) = end_id {
                        let r = self.popup.send(mgr, id, Event::Activate);
                        self.close_menu(mgr);
                        return r;
                    }
                }
            }
            event => return Response::Unhandled(event),
        }
        Response::None
    }
}

impl<W: Widget> event::SendEvent for MenuButton<W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.popup.id() {
            let r = self.popup.send(mgr, id, event);
            if r.is_msg() {
                self.close_menu(mgr);
            }
            r
        } else {
            Manager::handle_generic(self, mgr, event)
        }
    }
}

impl<W: Widget> HasText for MenuButton<W> {
    fn get_text(&self) -> &str {
        &self.label
    }

    fn set_cow_string(&mut self, text: CowString) -> TkAction {
        self.label = text;
        TkAction::Redraw
    }
}
