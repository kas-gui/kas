// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menus

use super::List;
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

    /// Open the pop-up menu, if not already open
    pub fn open_popup(&mut self, mgr: &mut Manager) {
        if self.popup_id.is_none() {
            let id = mgr.add_popup(kas::Popup {
                id: self.popup.id(),
                parent: self.id(),
                direction: Direction::Down,
            });
            self.popup_id = Some(id);
        }
    }

    /// Close the pop-up menu, if open
    pub fn close_popup(&mut self, mgr: &mut Manager) {
        if let Some(id) = self.popup_id {
            mgr.close_window(id);
            self.popup_id = None;
        }
    }
}

impl<M, W: Widget<Msg = M>> event::Handler for MenuButton<W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        match event {
            Event::Activate => {
                if self.popup_id.is_none() {
                    self.open_popup(mgr);
                } else {
                    self.close_popup(mgr);
                }
                Response::None
            }
            Event::PressStart { source, coord } if source.is_primary() => {
                mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                self.opening = self.popup_id.is_none();
                Response::None
            }
            // we deliberately leak some Unhandled move/end events for MenuBar
            Event::PressMove { coord, .. } if self.rect().contains(coord) => {
                self.open_popup(mgr);
                Response::None
            }
            Event::PressEnd { end_id, coord, .. } if self.rect().contains(coord) => {
                if end_id == Some(self.id()) && self.opening {
                    self.open_popup(mgr);
                } else {
                    self.close_popup(mgr);
                }
                Response::None
            }
            Event::PressEnd { end_id, coord, .. }
                if self.popup_id.is_some() && self.popup.rect().contains(coord) =>
            {
                if let Some(id) = end_id {
                    let r = self.popup.send(mgr, id, Event::Activate);
                    self.close_popup(mgr);
                    r
                } else {
                    Response::None
                }
            }
            Event::PressEnd {
                source,
                end_id,
                coord,
            } => {
                // We need to close our menu when not a child of a MenuBar,
                self.close_popup(mgr);
                // and allow the MenuBar to handle the event if we are:
                Response::Unhandled(Event::PressEnd {
                    source,
                    end_id,
                    coord,
                })
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
                self.close_popup(mgr);
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

/// A menubar
///
/// This widget houses a sequence of menu buttons, allowing input actions across
/// menus.
#[layout(single)]
#[handler(send=noauto, msg=M, generics=<M> where W: Widget<Msg = M>)]
#[derive(Clone, Debug, Widget)]
pub struct MenuBar<D: Directional, W: Widget> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub bar: List<D, MenuButton<W>>,
    active: usize,
}

impl<D: Directional + Default, W: Widget> MenuBar<D, W> {
    /// Construct
    pub fn new(menus: Vec<MenuButton<W>>) -> Self {
        MenuBar::new_with_direction(D::default(), menus)
    }
}

impl<D: Directional, W: Widget> MenuBar<D, W> {
    /// Construct
    pub fn new_with_direction(direction: D, menus: Vec<MenuButton<W>>) -> Self {
        MenuBar {
            core: Default::default(),
            bar: List::new_with_direction(direction, menus),
            active: 0,
        }
    }
}

impl<D: Directional, W: Widget> event::SendEvent for MenuBar<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.bar.id() {
            return match self.bar.send(mgr, id, event) {
                Response::Unhandled(event) => {
                    match event {
                        // HACK: code is tightly coupled with MenuButton,
                        // relying on leaking "Unhandled" events, and the
                        // result isn't quite correct.
                        Event::PressMove { coord, .. } => {
                            // We assume that a child requested a press grab
                            if self.rect().contains(coord) {
                                for i in 0..self.bar.len() {
                                    let w = &mut self.bar[i];
                                    if w.rect().contains(coord) {
                                        w.open_popup(mgr);
                                        self.active = i;
                                    } else {
                                        w.close_popup(mgr);
                                    }
                                }
                            }
                            Response::None
                        }
                        Event::PressEnd {
                            source,
                            coord,
                            end_id,
                        } => {
                            if let Some(id) = end_id {
                                if self.active < self.bar.len() {
                                    // Let the MenuButton's handler do the work
                                    let event = Event::PressEnd {
                                        source,
                                        coord,
                                        end_id,
                                    };
                                    self.bar[self.active].opening = true;
                                    return self.bar[self.active].send(mgr, id, event);
                                }
                            }
                            Response::None
                        }
                        e => Response::Unhandled(e),
                    }
                }
                r => r,
            };
        }

        debug_assert!(id == self.id(), "SendEvent::send: bad WidgetId");
        Manager::handle_generic(self, mgr, event)
    }
}
