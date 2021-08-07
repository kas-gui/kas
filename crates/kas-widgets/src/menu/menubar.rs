// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use super::{Menu, SubMenu};
use crate::List;
use kas::event::{self, Command, GrabMode};
use kas::prelude::*;

/// A menu-bar
///
/// This widget houses a sequence of menu buttons, allowing input actions across
/// menus.
#[derive(Clone, Debug, Widget)]
#[handler(noauto)]
pub struct MenuBar<W: Menu, D: Directional = kas::dir::Right> {
    #[widget_core]
    core: CoreData,
    #[widget]
    pub bar: List<D, SubMenu<D::Flipped, W>>,
    // Open mode. Used to close with click on root only when previously open.
    opening: bool,
    delayed_open: Option<WidgetId>,
}

impl<W: Menu, D: Directional + Default> MenuBar<W, D> {
    /// Construct a menubar
    ///
    /// Note: it appears that `MenuBar::new(..)` causes a type inference error,
    /// however `MenuBar::<_>::new(..)` does not. Alternatively one may specify
    /// the direction explicitly: `MenuBar::<_, kas::dir::Right>::new(..)`.
    pub fn new(menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        MenuBar::new_with_direction(D::default(), menus)
    }
}

impl<W: Menu, D: Directional> MenuBar<W, D> {
    /// Construct a menubar with explicit direction
    pub fn new_with_direction(direction: D, mut menus: Vec<SubMenu<D::Flipped, W>>) -> Self {
        for menu in menus.iter_mut() {
            menu.key_nav = false;
        }
        MenuBar {
            core: Default::default(),
            bar: List::new_with_direction(direction, menus),
            opening: false,
            delayed_open: None,
        }
    }
}

// NOTE: we could use layout(single) except for alignment
impl<W: Menu, D: Directional> Layout for MenuBar<W, D> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        self.bar.size_rules(size_handle, axis)
    }

    fn set_rect(&mut self, mgr: &mut Manager, rect: Rect, _: AlignHints) {
        self.core_data_mut().rect = rect;
        let align = AlignHints::new(Some(Align::Default), Some(Align::Default));
        self.bar.set_rect(mgr, rect, align);
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
impl<W: Menu<Msg = M>, D: Directional, M> event::Handler for MenuBar<W, D> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::TimerUpdate(0) => {
                if let Some(id) = self.delayed_open {
                    self.set_menu_path(mgr, Some(id));
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
                        self.find_leaf(start_id)
                            .map(|w| mgr.next_nav_focus(w, false, false));
                        self.opening = false;
                        let delay = mgr.config().menu_delay();
                        if self.rect().contains(coord) {
                            if let Some(w) = self.bar.iter().find(|w| w.id() == start_id) {
                                if !w.menu_is_open() {
                                    self.opening = true;
                                    self.delayed_open = Some(start_id);
                                    mgr.update_on_timer(delay, self.id(), 0);
                                }
                            }
                        } else {
                            self.delayed_open = Some(start_id);
                            mgr.update_on_timer(delay, self.id(), 0);
                        }
                    }
                } else {
                    self.delayed_open = None;
                    return Response::Unhandled;
                }
            }
            Event::PressMove {
                source,
                cur_id,
                coord,
                ..
            } => {
                mgr.set_grab_depress(source, cur_id);
                if let Some(id) = cur_id {
                    if self.is_ancestor_of(id) {
                        mgr.set_nav_focus(id, false);
                        // We instantly open a sub-menu on motion over the bar,
                        // but delay when over a sub-menu (most intuitive?)
                        if self.rect().contains(coord) {
                            self.set_menu_path(mgr, Some(id));
                        } else {
                            self.delayed_open = Some(id);
                            let delay = mgr.config().menu_delay();
                            mgr.update_on_timer(delay, self.id(), 0);
                        }
                    }
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
                                    self.bar[i].set_menu_path(mgr, None);
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
                    self.set_menu_path(mgr, None);
                }
            }
            Event::Command(cmd, _) => {
                // Arrow keys can switch to the next / previous menu.
                let is_vert = self.bar.direction().is_vertical();
                let reverse = self.bar.direction().is_reversed()
                    ^ match cmd {
                        Command::Left if !is_vert => true,
                        Command::Right if !is_vert => false,
                        Command::Up if is_vert => true,
                        Command::Down if is_vert => false,
                        _ => return Response::Unhandled,
                    };

                for i in 0..self.bar.len() {
                    if self.bar[i].menu_is_open() {
                        let index = if reverse { i.wrapping_sub(1) } else { i + 1 };
                        if index < self.bar.len() {
                            self.bar[i].set_menu_path(mgr, None);
                            let w = &mut self.bar[index];
                            w.set_menu_path(mgr, Some(w.id()));
                        }
                        break;
                    }
                }
            }
            _ => return Response::Unhandled,
        }
        Response::None
    }
}

impl<W: Menu, D: Directional> event::SendEvent for MenuBar<W, D> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled;
        }

        if id <= self.bar.id() {
            return match self.bar.send(mgr, id, event.clone()) {
                Response::Unhandled => self.handle(mgr, event),
                r => r.try_into().unwrap_or_else(|(_, msg)| {
                    log::trace!(
                        "Received by {} from {}: {:?}",
                        self.id(),
                        id,
                        kas::util::TryFormat(&msg)
                    );
                    Response::Msg(msg)
                }),
            };
        }

        self.handle(mgr, event)
    }
}

impl<W: Menu, D: Directional> Menu for MenuBar<W, D> {
    fn set_menu_path(&mut self, mgr: &mut Manager, target: Option<WidgetId>) {
        self.delayed_open = None;
        if let Some(id) = target {
            // We should close other sub-menus before opening
            let mut child = None;
            for i in 0..self.bar.len() {
                if self.bar[i].is_ancestor_of(id) {
                    child = Some(i);
                } else {
                    self.bar[i].set_menu_path(mgr, None);
                }
            }
            if let Some(i) = child {
                self.bar[i].set_menu_path(mgr, target);
            }
        } else {
            for i in 0..self.bar.len() {
                self.bar[i].set_menu_path(mgr, None);
            }
        }
    }
}
