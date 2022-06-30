// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use super::{Menu, SubMenu, SubMenuBuilder};
use kas::event::Command;
use kas::layout::{self, RowPositionSolver, RowSetter, RowSolver, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::FrameStyle;

impl_scope! {
    /// A menu-bar
    ///
    /// This widget houses a sequence of menu buttons, allowing input actions across
    /// menus.
    #[autoimpl(Debug where D: trait)]
    #[widget]
    pub struct MenuBar<D: Directional = kas::dir::Right> {
        core: widget_core!(),
        direction: D,
        widgets: Vec<SubMenu<D::Flipped>>,
        layout_store: layout::DynRowStorage,
        delayed_open: Option<WidgetId>,
    }

    impl Self where D: Default {
        /// Construct a menubar
        ///
        /// Note: it appears that `MenuBar::new(..)` causes a type inference error,
        /// however `MenuBar::<_>::new(..)` does not. Alternatively one may specify
        /// the direction explicitly: `MenuBar::<_, kas::dir::Right>::new(..)`.
        pub fn new(menus: Vec<SubMenu<D::Flipped>>) -> Self {
            MenuBar::new_with_direction(D::default(), menus)
        }
    }

    impl Self {
        /// Construct a menubar with explicit direction
        pub fn new_with_direction(direction: D, mut menus: Vec<SubMenu<D::Flipped>>) -> Self {
            for menu in menus.iter_mut() {
                menu.key_nav = false;
            }
            MenuBar {
                core: Default::default(),
                direction,
                widgets: menus,
                layout_store: Default::default(),
                delayed_open: None,
            }
        }

        pub fn builder() -> MenuBuilder<D> {
            MenuBuilder { menus: vec![] }
        }
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.widgets.get(index).map(|w| w.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(index).map(|w| w.as_widget_mut())
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let dim = (self.direction, self.widgets.len());
            let mut solver = RowSolver::new(axis, dim, &mut self.layout_store);
            let frame_rules = mgr.frame(FrameStyle::MenuEntry, axis);
            for (n, child) in self.widgets.iter_mut().enumerate() {
                solver.for_child(&mut self.layout_store, n, |axis| {
                    let rules = child.size_rules(mgr.re(), axis);
                    frame_rules.surround_as_margin(rules).0
                });
            }
            solver.finish(&mut self.layout_store)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let dim = (self.direction, self.widgets.len());
            let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, &mut self.layout_store);

            for (n, child) in self.widgets.iter_mut().enumerate() {
                child.set_rect(mgr, setter.child_rect(&mut self.layout_store, n), AlignHints::CENTER);
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            // TODO: doesn't check self.rect() first!
            let solver = RowPositionSolver::new(self.direction);
            solver
                .find_child_mut(&mut self.widgets, coord)
                .and_then(|child| child.find_id(coord))
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let solver = RowPositionSolver::new(self.direction);
            solver.for_children(&mut self.widgets, self.core.rect, |w| draw.recurse(w));
        }
    }

    impl<D: Directional> Widget for MenuBar<D> {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::TimerUpdate(id_code) => {
                    if let Some(id) = self.delayed_open.clone() {
                        if id.as_u64() == id_code {
                            self.set_menu_path(mgr, Some(&id), false);
                        }
                    }
                    Response::Used
                }
                Event::PressStart {
                    source,
                    start_id,
                    coord,
                } => {
                    if start_id.as_ref().map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                        if source.is_primary() {
                            let any_menu_open = self.widgets.iter().any(|w| w.menu_is_open());
                            let press_in_the_bar = self.rect().contains(coord);

                            if !press_in_the_bar || !any_menu_open {
                                mgr.grab_press_unique(self.id(), source, coord, None);
                            }
                            mgr.set_grab_depress(source, start_id.clone());
                            if press_in_the_bar {
                                if self
                                    .widgets
                                    .iter()
                                    .any(|w| w.eq_id(&start_id) && !w.menu_is_open())
                                {
                                    self.set_menu_path(mgr, start_id.as_ref(), false);
                                } else {
                                    self.set_menu_path(mgr, None, false);
                                }
                            }
                        }
                        Response::Used
                    } else {
                        // Click happened out of the menubar or submenus,
                        // while one or more submenus are opened.
                        self.delayed_open = None;
                        Response::Unused
                    }
                }
                Event::PressMove {
                    source,
                    cur_id,
                    coord,
                    ..
                } => {
                    mgr.set_grab_depress(source, cur_id.clone());

                    let id = match cur_id {
                        Some(x) => x,
                        None => return Response::Used,
                    };

                    if self.is_strict_ancestor_of(&id) {
                        // We instantly open a sub-menu on motion over the bar,
                        // but delay when over a sub-menu (most intuitive?)
                        if self.rect().contains(coord) {
                            mgr.clear_nav_focus();
                            self.delayed_open = None;
                            self.set_menu_path(mgr, Some(&id), false);
                        } else if id != self.delayed_open {
                            mgr.set_nav_focus(id.clone(), false);
                            let delay = mgr.config().menu_delay();
                            mgr.request_update(self.id(), id.as_u64(), delay, true);
                            self.delayed_open = Some(id);
                        }
                    } else {
                        self.delayed_open = None;
                    }
                    Response::Used
                }
                Event::PressEnd { coord, end_id, success, .. } if success => {
                    let id = match end_id {
                        Some(x) => x,
                        None => return Response::Used,
                    };

                    if !self.rect().contains(coord) {
                        // not on the menubar
                        self.delayed_open = None;
                        return mgr.send(self, id, Event::Command(Command::Activate));
                    }
                    Response::Used
                }
                Event::PressEnd { .. } => Response::Used,
                Event::Command(cmd) => {
                    // Arrow keys can switch to the next / previous menu
                    // as well as to the first / last item of an open menu.
                    use Command::{Left, Up};
                    let is_vert = self.direction.is_vertical();
                    let reverse = self.direction.is_reversed() ^ matches!(cmd, Left | Up);
                    match cmd.as_direction().map(|d| d.is_vertical()) {
                        Some(v) if v == is_vert => {
                            for i in 0..self.widgets.len() {
                                if self.widgets[i].menu_is_open() {
                                    let mut j = isize::conv(i);
                                    j = if reverse { j - 1 } else { j + 1 };
                                    j = j.rem_euclid(self.widgets.len().cast());
                                    self.widgets[i].set_menu_path(mgr, None, true);
                                    let w = &mut self.widgets[usize::conv(j)];
                                    w.set_menu_path(mgr, Some(&w.id()), true);
                                    break;
                                }
                            }
                            Response::Used
                        }
                        Some(_) => {
                            mgr.next_nav_focus(self, reverse, true);
                            Response::Used
                        }
                        None => Response::Unused,
                    }
                }
                _ => Response::Unused,
            }
        }
    }

    impl Self {
        fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
            log::trace!("{}::set_menu_path: target={:?}, set_focus={}", self.identify(), target, set_focus);
            self.delayed_open = None;
            for i in 0..self.widgets.len() {
                self.widgets[i].set_menu_path(mgr, target, set_focus);
            }
        }
    }
}

/// Builder for [`MenuBar`]
///
/// Access through [`MenuBar::builder`].
pub struct MenuBuilder<D: Directional> {
    menus: Vec<SubMenu<D::Flipped>>,
}

impl<D: Directional> MenuBuilder<D> {
    /// Add a new menu
    ///
    /// The menu's direction is determined via [`Directional::Flipped`].
    pub fn menu<F>(mut self, label: impl Into<AccelString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder),
        D::Flipped: Default,
    {
        let mut menu = Vec::new();
        f(SubMenuBuilder { menu: &mut menu });
        self.menus.push(SubMenu::new(label, menu));
        self
    }

    /// Finish, yielding a [`MenuBar`]
    pub fn build(self) -> MenuBar<D>
    where
        D: Default,
    {
        MenuBar::new(self.menus)
    }
}
