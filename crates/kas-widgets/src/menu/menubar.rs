// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Menubar

use super::{Menu, SubMenu, SubMenuBuilder};
use kas::event::{Command, FocusSource, TimerHandle};
use kas::layout::{self, RowPositionSolver, RowSetter, RowSolver, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::FrameStyle;

const TIMER_SHOW: TimerHandle = TimerHandle::new(0, false);

#[impl_self]
mod MenuBar {
    /// A menu-bar
    ///
    /// This widget houses a sequence of menu buttons, allowing input actions across
    /// menus.
    #[widget]
    pub struct MenuBar<Data, D: Directional = kas::dir::Right> {
        core: widget_core!(),
        direction: D,
        widgets: Vec<SubMenu<true, Data>>,
        layout_store: layout::DynRowStorage,
        delayed_open: Option<Id>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a menubar
        pub fn new(menus: Vec<SubMenu<true, Data>>) -> Self {
            MenuBar::new_dir(menus, Default::default())
        }

        /// Construct a menu builder
        pub fn builder() -> MenuBuilder<Data, D> {
            MenuBuilder {
                menus: vec![],
                direction: D::default(),
            }
        }
    }
    impl<Data> MenuBar<Data, kas::dir::Right> {
        /// Construct a menubar
        pub fn right(menus: Vec<SubMenu<true, Data>>) -> Self {
            MenuBar::new(menus)
        }
    }

    impl Self {
        /// Construct a menubar with explicit direction
        pub fn new_dir(menus: Vec<SubMenu<true, Data>>, direction: D) -> Self {
            MenuBar {
                core: Default::default(),
                direction,
                widgets: menus,
                layout_store: Default::default(),
                delayed_open: None,
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            // Unusual behaviour: children's SizeRules are padded with a frame,
            // but the frame does not adjust the children's rects.

            let len = self.widgets.len();
            let dim = (self.direction, len + 1);
            let mut solver = RowSolver::new(axis, dim, &mut self.layout_store);
            let frame_rules = sizer.frame(FrameStyle::MenuEntry, axis);
            for (n, child) in self.widgets.iter_mut().enumerate() {
                solver.for_child(&mut self.layout_store, n, |axis| {
                    let rules = child.size_rules(sizer.re(), axis);
                    frame_rules.surround(rules).0
                });
            }
            solver.for_child(&mut self.layout_store, len, |axis| {
                let mut rules = SizeRules::EMPTY;
                if axis.is_horizontal() {
                    rules.set_stretch(Stretch::Maximize);
                }
                rules
            });
            solver.finish(&mut self.layout_store)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, _: AlignHints) {
            widget_set_rect!(rect);
            let dim = (self.direction, self.widgets.len() + 1);
            let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, &mut self.layout_store);
            let hints = AlignHints::CENTER;

            for (n, child) in self.widgets.iter_mut().enumerate() {
                child.set_rect(cx, setter.child_rect(&mut self.layout_store, n), hints);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let solver = RowPositionSolver::new(self.direction);
            let rect = self.rect();
            solver.for_children(&self.widgets, rect, |w| w.draw(draw.re()));
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::MenuBar
        }

        #[inline]
        fn child_indices(&self) -> ChildIndices {
            (0..self.widgets.len()).into()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets.get(index).map(|w| w.as_tile())
        }

        fn probe(&self, coord: Coord) -> Id {
            let solver = RowPositionSolver::new(self.direction);
            solver
                .find_child(&self.widgets, coord)
                .and_then(|child| child.try_probe(coord))
                .unwrap_or_else(|| self.id())
        }
    }

    impl Events for Self {
        fn handle_event(&mut self, cx: &mut EventCx, data: &Data, event: Event) -> IsUsed {
            match event {
                Event::Timer(TIMER_SHOW) => {
                    if let Some(id) = self.delayed_open.clone() {
                        self.set_menu_path(cx, data, Some(&id), false);
                    }
                    Used
                }
                Event::PressStart(press) => {
                    if press
                        .id
                        .as_ref()
                        .map(|id| self.is_ancestor_of(id))
                        .unwrap_or(false)
                    {
                        if press.is_primary() {
                            let any_menu_open = self.widgets.iter().any(|w| w.menu_is_open());
                            let press_in_the_bar = self.rect().contains(press.coord);

                            if !press_in_the_bar || !any_menu_open {
                                press.grab_move(self.id()).complete(cx);
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
                        Used
                    } else {
                        // Click happened out of the menubar or submenus,
                        // while one or more submenus are opened.
                        self.delayed_open = None;
                        self.set_menu_path(cx, data, None, false);
                        Unused
                    }
                }
                Event::CursorMove { press } | Event::PressMove { press, .. } => {
                    cx.set_grab_depress(*press, press.id.clone());

                    let id = match press.id {
                        Some(x) => x,
                        None => return Used,
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
                            let delay = cx.config().event().menu_delay();
                            cx.request_timer(self.id(), TIMER_SHOW, delay);
                            self.delayed_open = Some(id);
                        }
                    } else {
                        self.delayed_open = None;
                    }
                    Used
                }
                Event::PressEnd { press, success, .. } if success => {
                    let id = match press.id {
                        Some(x) => x,
                        None => return Used,
                    };

                    if !self.rect().contains(press.coord) {
                        // not on the menubar
                        self.delayed_open = None;
                        cx.send(id, Command::Activate);
                    }
                    Used
                }
                Event::Command(cmd, _) => {
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
                            Used
                        }
                        Some(_) => {
                            cx.next_nav_focus(self.id(), reverse, FocusSource::Key);
                            Used
                        }
                        None => Unused,
                    }
                }
                _ => Unused,
            }
        }
    }

    impl Widget for Self {
        type Data = Data;

        fn child_node<'n>(&'n mut self, data: &'n Data, index: usize) -> Option<Node<'n>> {
            self.widgets.get_mut(index).map(|w| w.as_node(data))
        }
    }

    impl Self {
        fn set_menu_path(
            &mut self,
            cx: &mut EventCx,
            data: &Data,
            target: Option<&Id>,
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
    menus: Vec<SubMenu<true, Data>>,
    direction: D,
}

impl<Data, D: Directional> MenuBuilder<Data, D> {
    /// Add a new menu
    ///
    /// The menu's direction is determined via [`Directional::Flipped`].
    pub fn menu<F>(mut self, label: impl Into<AccessString>, f: F) -> Self
    where
        F: FnOnce(SubMenuBuilder<Data>),
    {
        let mut menu = Vec::new();
        f(SubMenuBuilder { menu: &mut menu });
        let dir = self.direction.as_direction().flipped();
        self.menus.push(SubMenu::new(label, menu, dir));
        self
    }

    /// Finish, yielding a [`MenuBar`]
    pub fn build(self) -> MenuBar<Data, D> {
        MenuBar::new_dir(self.menus, self.direction)
    }
}
