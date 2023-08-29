// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use super::{Menu, SubMenu, SubMenuBuilder};
use kas::event::{Command, FocusSource};
use kas::layout::{self, RowPositionSolver, RowSetter, RowSolver, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::FrameStyle;

impl_scope! {
    /// A menu-bar
    ///
    /// This widget houses a sequence of menu buttons, allowing input actions across
    /// menus.
    #[widget]
    pub struct MenuBar<Data, D: Directional = kas::dir::Right> {
        core: widget_core!(),
        direction: D,
        widgets: Vec<SubMenu<Data, D::Flipped>>,
        layout_store: layout::DynRowStorage,
        delayed_open: Option<WidgetId>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a menubar
        pub fn new(menus: Vec<SubMenu<Data, D::Flipped>>) -> Self {
            MenuBar::new_dir(menus, Default::default())
        }

        /// Construct a menu builder
        pub fn builder() -> MenuBuilder<Data, D> {
            MenuBuilder { menus: vec![], direction: D::default() }
        }
    }
    impl<Data> MenuBar<Data, kas::dir::Right> {
        /// Construct a menubar
        pub fn right(menus: Vec<SubMenu<Data, kas::dir::Down>>) -> Self {
            MenuBar::new(menus)
        }
    }

    impl Self {
        /// Construct a menubar with explicit direction
        pub fn new_dir(mut menus: Vec<SubMenu<Data, D::Flipped>>, direction: D) -> Self {
            for menu in menus.iter_mut() {
                menu.navigable = false;
            }
            MenuBar {
                core: Default::default(),
                direction,
                widgets: menus,
                layout_store: Default::default(),
                delayed_open: None,
            }
        }
    }

    impl Widget for Self {
        fn for_child_node(
            &mut self,
            data: &Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                closure(w.as_node(data));
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.widgets.get(index).map(|w| w.as_layout())
        }

        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            // Unusual behaviour: children's SizeRules are padded with a frame,
            // but the frame does not adjust the children's rects.

            axis.set_default_align(Align::Center);
            let dim = (self.direction, self.widgets.len());
            let mut solver = RowSolver::new(axis, dim, &mut self.layout_store);
            let frame_rules = sizer.frame(FrameStyle::MenuEntry, axis);
            for (n, child) in self.widgets.iter_mut().enumerate() {
                solver.for_child(&mut self.layout_store, n, |axis| {
                    let rules = child.size_rules(sizer.re(), axis);
                    frame_rules.surround(rules).0
                });
            }
            solver.finish(&mut self.layout_store)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            let dim = (self.direction, self.widgets.len());
            let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, &mut self.layout_store);

            for (n, child) in self.widgets.iter_mut().enumerate() {
                child.set_rect(cx, setter.child_rect(&mut self.layout_store, n));
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }
            let solver = RowPositionSolver::new(self.direction);
            solver
                .find_child_mut(&mut self.widgets, coord)
                .and_then(|child| child.find_id(coord))
                .or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawCx) {
            let solver = RowPositionSolver::new(self.direction);
            solver.for_children(&mut self.widgets, self.core.rect, |w| draw.recurse(w));
        }
    }

    impl<Data, D: Directional> Events for MenuBar<Data, D> {
        type Data = Data;

        fn handle_event(&mut self, cx: &mut EventCx, data: &Data, event: Event) -> Response {
            match event {
                Event::TimerUpdate(id_code) => {
                    if let Some(id) = self.delayed_open.clone() {
                        if id.as_u64() == id_code {
                            self.set_menu_path(cx, data, Some(&id), false);
                        }
                    }
                    Response::Used
                }
                Event::PressStart { press } => {
                    if press.id
                        .as_ref()
                        .map(|id| self.is_ancestor_of(id))
                        .unwrap_or(false)
                    {
                        if press.is_primary() {
                            let any_menu_open = self.widgets.iter().any(|w| w.menu_is_open());
                            let press_in_the_bar = self.rect().contains(press.coord);

                            if !press_in_the_bar || !any_menu_open {
                                press.grab(self.id()).with_cx(cx);
                            }
                            cx.set_grab_depress(*press, press.id.clone());
                            if press_in_the_bar {
                                if self
                                    .widgets
                                    .iter()
                                    .any(|w| w.eq_id(&press.id) && !w.menu_is_open())
                                {
                                    self.set_menu_path(cx, data, press.id.as_ref(), false);
                                } else {
                                    self.set_menu_path(cx, data, None, false);
                                }
                            }
                        }
                        Response::Used
                    } else {
                        // Click happened out of the menubar or submenus,
                        // while one or more submenus are opened.
                        self.delayed_open = None;
                        self.set_menu_path(cx, data, None, false);
                        Response::Unused
                    }
                }
                Event::CursorMove { press } | Event::PressMove { press, .. } => {
                    cx.set_grab_depress(*press, press.id.clone());

                    let id = match press.id {
                        Some(x) => x,
                        None => return Response::Used,
                    };

                    if self.is_strict_ancestor_of(&id) {
                        // We instantly open a sub-menu on motion over the bar,
                        // but delay when over a sub-menu (most intuitive?)
                        if self.rect().contains(press.coord) {
                            cx.clear_nav_focus();
                            self.delayed_open = None;
                            self.set_menu_path(cx, data, Some(&id), false);
                        } else if id != self.delayed_open {
                            cx.set_nav_focus(id.clone(), FocusSource::Pointer);
                            let delay = cx.config().menu_delay();
                            cx.request_timer_update(self.id(), id.as_u64(), delay, true);
                            self.delayed_open = Some(id);
                        }
                    } else {
                        self.delayed_open = None;
                    }
                    Response::Used
                }
                Event::PressEnd {
                    press,
                    success,
                    ..
                } if success => {
                    let id = match press.id {
                        Some(x) => x,
                        None => return Response::Used,
                    };

                    if !self.rect().contains(press.coord) {
                        // not on the menubar
                        self.delayed_open = None;
                        cx.send(id, Event::Command(Command::Activate));
                    }
                    Response::Used
                }
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
                                    self.widgets[i].set_menu_path(cx, data, None, true);
                                    let w = &mut self.widgets[usize::conv(j)];
                                    w.set_menu_path(cx, data, Some(&w.id()), true);
                                    break;
                                }
                            }
                            Response::Used
                        }
                        Some(_) => {
                            cx.next_nav_focus(self.id(), reverse, FocusSource::Key);
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
        fn set_menu_path(
            &mut self,
            cx: &mut EventCx,
            data: &Data,
            target: Option<&WidgetId>,
            set_focus: bool,
        ) {
            log::trace!(
                "set_menu_path: self={}, target={target:?}, set_focus={set_focus}",
                self.identify()
            );
            self.delayed_open = None;
            for i in 0..self.widgets.len() {
                self.widgets[i].set_menu_path(cx, data, target, set_focus);
            }
        }
    }
}

/// Builder for [`MenuBar`]
///
/// Access through [`MenuBar::builder`].
pub struct MenuBuilder<Data, D: Directional> {
    menus: Vec<SubMenu<Data, D::Flipped>>,
    direction: D,
}

impl<Data, D: Directional> MenuBuilder<Data, D> {
    /// Add a new menu
    ///
    /// The menu's direction is determined via [`Directional::Flipped`].
    pub fn menu<F>(mut self, label: impl Into<AccelString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder<Data>),
        D::Flipped: Default,
    {
        let mut menu = Vec::new();
        f(SubMenuBuilder { menu: &mut menu });
        self.menus.push(SubMenu::new(label, menu));
        self
    }

    /// Finish, yielding a [`MenuBar`]
    pub fn build(self) -> MenuBar<Data, D> {
        MenuBar::new_dir(self.menus, self.direction)
    }
}
