// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use smallvec::SmallVec;
use std::time::Duration;

use super::SubMenu;
use kas::draw::{DrawHandle, SizeHandle};
use kas::event::{Event, GrabMode, Handler, Manager, Response, SendEvent};
use kas::layout::{AxisInfo, SizeRules};
use kas::prelude::*;
use kas::widget::List;

/// A menu-bar
///
/// This widget houses a sequence of menu buttons, allowing input actions across
/// menus.
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct MenuBar<D: Directional, W: Widget> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub bar: List<D, SubMenu<D::Flipped, W>>,
    // Open mode. Used to close with click on root only when previously open.
    opening: bool,
    // Stack of open child windows. Each should be a descendant of the previous.
    open: SmallVec<[WidgetId; 16]>,
    delayed_open: Option<WidgetId>,
}

impl<D: Directional + Default, W: Widget> MenuBar<D, W> {
    /// Construct
    pub fn new(menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        MenuBar::new_with_direction(D::default(), menus)
    }
}

impl<D: Directional, W: Widget> MenuBar<D, W> {
    /// Construct
    pub fn new_with_direction(direction: D, menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        MenuBar {
            core: Default::default(),
            bar: List::new_with_direction(direction, menus),
            opening: false,
            open: Default::default(),
            delayed_open: None,
        }
    }
}

// NOTE: we could use layout(single) except for alignment
impl<D: Directional, W: Widget> Layout for MenuBar<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.bar.size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, rect: Rect, _: AlignHints) {
        self.core_data_mut().rect = rect;
        let align = AlignHints::new(Some(Align::Begin), Some(Align::Begin));
        self.bar.set_rect(rect, align);
    }

    #[inline]
    fn find_id(&self, coord: Coord) -> Option<WidgetId> {
        if !self.rect().contains(coord) {
            return None;
        }
        if let Some(id) = self.bar.find_id(coord) {
            return Some(id);
        }
        Some(self.id())
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        self.bar.draw(draw_handle, mgr, disabled);
    }
}
impl<D: Directional, W: Widget<Msg = M>, M> event::Handler for MenuBar<D, W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::TimerUpdate => {
                if let Some(id) = self.delayed_open {
                    self.delayed_open = None;
                    while let Some(last_id) = self.open.last().cloned() {
                        if !self.find(last_id).map(|w| w.is_ancestor_of(id)).unwrap() {
                            let _ = self.send(mgr, last_id, Event::ClosePopup);
                            self.open.pop();
                        } else {
                            break;
                        }
                    }
                    self.open.push(id);
                    return self.send(mgr, id, Event::OpenPopup);
                }
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => {
                if self.is_ancestor_of(start_id) {
                    if source.is_primary()
                        && mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None)
                    {
                        mgr.set_grab_depress(source, Some(start_id));
                        self.find(start_id).map(|w| mgr.next_nav_focus(w, false));
                        self.opening = false;
                        if self.rect().contains(coord) {
                            // We could just send Event::OpenPopup, but we also
                            // need to set self.opening
                            for i in 0..self.bar.len() {
                                let w = &mut self.bar[i];
                                let id = w.id();
                                if id == start_id {
                                    if !w.menu_is_open() {
                                        self.opening = true;
                                        self.delayed_open = Some(id);
                                        mgr.update_on_timer(Duration::from_millis(100), self.id());
                                    }
                                    break;
                                }
                            }
                        } else {
                            self.delayed_open = Some(start_id);
                            mgr.update_on_timer(Duration::from_millis(100), self.id());
                        }
                    }
                } else {
                    while let Some(id) = self.open.pop() {
                        let _ = self.send(mgr, id, Event::ClosePopup);
                    }
                    return Response::Unhandled(Event::None);
                }
            }
            Event::PressMove { source, cur_id, .. } => {
                if let Some(w) = cur_id.and_then(|id| self.find(id)) {
                    if w.key_nav() {
                        let id = cur_id.unwrap();
                        mgr.set_grab_depress(source, Some(id));
                        mgr.set_nav_focus(id);
                        self.delayed_open = Some(id);
                        mgr.update_on_timer(Duration::from_millis(300), self.id());
                    }
                } else {
                    mgr.set_grab_depress(source, None);
                    mgr.clear_nav_focus();
                }
            }
            Event::PressEnd { coord, end_id, .. } => {
                if end_id.map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                    // end_id is a child of self
                    let id = end_id.unwrap();

                    if self.rect().contains(coord) {
                        if !self.opening {
                            while let Some(id) = self.open.pop() {
                                let _ = self.send(mgr, id, Event::ClosePopup);
                            }
                        }
                    } else {
                        return self.send(mgr, id, Event::Activate);
                    }
                } else {
                    while let Some(id) = self.open.pop() {
                        let _ = self.send(mgr, id, Event::ClosePopup);
                    }
                }
            }
            e => return Response::Unhandled(e),
        }
        Response::None
    }
}

impl<D: Directional, W: Widget> event::SendEvent for MenuBar<D, W> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.bar.id() {
            return match self.bar.send(mgr, id, event) {
                Response::Unhandled(event) => self.handle(mgr, event),
                r => r,
            };
        }

        self.handle(mgr, event)
    }
}
