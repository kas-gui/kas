// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menus

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
    // text_rect: Rect,
    label: CowString,
    #[widget]
    popup: W,
    opening: bool,
    popup_id: Option<WindowId>,
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

        // In theory, text rendering should be restricted as in EditBox.
        // In practice, it sometimes overflows a tiny bit, and looks better if
        // we let it overflow. Since the text is centred this is okay.
        // self.text_rect = ...
    }

    fn spatial_range(&self) -> (usize, usize) {
        // We have no child within our rect; return an empty range
        (0, std::usize::MAX)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        draw_handle.button(self.core.rect, self.input_state(mgr, disabled));
        let align = (Align::Centre, Align::Centre);
        draw_handle.text(self.core.rect, &self.label, TextClass::Button, align);
    }
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
}

impl<M, W: Widget<Msg = M>> event::Handler for MenuButton<W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        let open_popup = |w: &mut MenuButton<W>, mgr: &mut Manager| {
            let id = mgr.add_popup(kas::Popup {
                id: w.popup.id(),
                parent: w.id(),
                direction: Direction::Down,
            });
            w.popup_id = Some(id);
        };
        match event {
            Event::Activate => {
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                    self.popup_id = None;
                } else {
                    open_popup(self, mgr);
                }
                Response::None
            }
            Event::PressStart { source, coord } if source.is_primary() => {
                mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                self.opening = self.popup_id.is_none();
                Response::None
            }
            Event::PressMove { .. } => {
                if self.popup_id.is_none() {
                    open_popup(self, mgr);
                }
                Response::None
            }
            Event::PressEnd { end_id, coord, .. } => {
                if let Some(id) = end_id {
                    if id == self.id() {
                        if self.opening {
                            if self.popup_id.is_none() {
                                open_popup(self, mgr);
                            }
                            return Response::None;
                        }
                    } else if let Some(wid) = self.popup_id {
                        if self.popup.rect().contains(coord) {
                            let r = self.popup.send(mgr, id, Event::Activate);
                            if r.is_msg() {
                                mgr.close_window(wid);
                                self.popup_id = None;
                            }
                            return r;
                        }
                    }
                }
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                    self.popup_id = None;
                }
                Response::None
            }
            event => Response::Unhandled(event),
        }
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
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                    self.popup_id = None;
                }
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
