// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use std::time::Duration;

use super::{Menu, SubMenu};
use kas::event::{ControlKey, GrabMode};
use kas::prelude::*;
use kas::widget::List;

const DELAY: Duration = Duration::from_millis(200);

/// A menu-bar
///
/// This widget houses a sequence of menu buttons, allowing input actions across
/// menus.
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct MenuBar<D: Directional, W: Menu> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub bar: List<D, SubMenu<D::Flipped, W>>,
    // Open mode. Used to close with click on root only when previously open.
    opening: bool,
    delayed_open: Option<WidgetId>,
}

impl<D: Directional + Default, W: Menu> MenuBar<D, W> {
    /// Construct
    pub fn new(menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        MenuBar::new_with_direction(D::default(), menus)
    }
}

impl<D: Directional, W: Menu> MenuBar<D, W> {
    /// Construct
    pub fn new_with_direction(direction: D, menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        MenuBar {
            core: Default::default(),
            bar: List::new_with_direction(direction, menus),
            opening: false,
            delayed_open: None,
        }
    }
}

// NOTE: we could use layout(single) except for alignment
impl<D: Directional, W: Menu> Layout for MenuBar<D, W> {
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
impl<D: Directional, W: Menu<Msg = M>, M> event::Handler for MenuBar<D, W> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::TimerUpdate => {
                if let Some(id) = self.delayed_open {
                    self.delayed_open = None;
                    self.menu_path(mgr, Some(id));
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
                                        mgr.update_on_timer(DELAY, self.id());
                                    }
                                    break;
                                }
                            }
                        } else {
                            self.delayed_open = Some(start_id);
                            mgr.update_on_timer(DELAY, self.id());
                        }
                    }
                } else {
                    self.delayed_open = None;
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
                        mgr.update_on_timer(DELAY, self.id());
                    }
                } else {
                    mgr.set_grab_depress(source, None);
                }
            }
            Event::PressEnd { coord, end_id, .. } => {
                if end_id.map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                    // end_id is a child of self
                    let id = end_id.unwrap();

                    if self.rect().contains(coord) {
                        // end coordinate is on the menubar
                        if !self.opening {
                            self.delayed_open = None;
                            for i in 0..self.bar.len() {
                                if self.bar[i].id() == id {
                                    self.bar[i].menu_path(mgr, None);
                                }
                            }
                        }
                    } else {
                        // not on the menubar, thus on a sub-menu
                        self.delayed_open = None;
                        return self.send(mgr, id, Event::Activate);
                    }
                } else {
                    // not on the menu
                    self.delayed_open = None;
                    self.menu_path(mgr, None);
                }
            }
            Event::Control(key) => {
                // Arrow keys can switch to the next / previous menu.
                let is_vert = self.bar.direction().is_vertical();
                let reverse = self.bar.direction().is_reversed()
                    ^ match key {
                        ControlKey::Left if !is_vert => true,
                        ControlKey::Right if !is_vert => false,
                        ControlKey::Up if is_vert => true,
                        ControlKey::Down if is_vert => false,
                        key => return Response::Unhandled(Event::Control(key)),
                    };

                for i in 0..self.bar.len() {
                    if self.bar[i].menu_is_open() {
                        let index = if reverse { i.wrapping_sub(1) } else { i + 1 };
                        if index < self.bar.len() {
                            self.delayed_open = None;
                            self.bar[i].menu_path(mgr, None);
                            let w = &mut self.bar[index];
                            w.menu_path(mgr, Some(w.id()));
                        }
                        break;
                    }
                }
            }
            e => return Response::Unhandled(e),
        }
        Response::None
    }
}

impl<D: Directional, W: Menu> event::SendEvent for MenuBar<D, W> {
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

impl<D: Directional, W: Menu> Menu for MenuBar<D, W> {
    fn menu_path(&mut self, mgr: &mut Manager, target: Option<WidgetId>) {
        if let Some(id) = target {
            // We should close other sub-menus before opening
            let mut child = None;
            for i in 0..self.bar.len() {
                if self.bar[i].is_ancestor_of(id) {
                    child = Some(i);
                } else {
                    self.bar[i].menu_path(mgr, None);
                }
            }
            if let Some(i) = child {
                self.bar[i].menu_path(mgr, target);
            }
        } else {
            for i in 0..self.bar.len() {
                self.bar[i].menu_path(mgr, None);
            }
        }
    }
}
