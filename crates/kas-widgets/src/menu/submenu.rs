// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::{BoxedMenu, Menu, SubItems};
use crate::PopupFrame;
use kas::component::{Component, Label, Mark};
use kas::event::Command;
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::{FrameStyle, MarkStyle, TextClass};
use kas::WindowId;

impl_scope! {
    /// A sub-menu
    #[autoimpl(Debug where D: trait)]
    #[widget]
    pub struct SubMenu<D: Directional> {
        #[widget_core]
        core: CoreData,
        direction: D,
        pub(crate) key_nav: bool,
        label: Label<AccelString>,
        mark: Mark,
        #[widget]
        list: PopupFrame<MenuView<BoxedMenu>>,
        popup_id: Option<WindowId>,
    }

    impl Self where D: Default {
        /// Construct a sub-menu
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu>) -> Self {
            SubMenu::new_with_direction(Default::default(), label, list)
        }
    }

    impl SubMenu<kas::dir::Right> {
        /// Construct a sub-menu, opening to the right
        // NOTE: this is used since we can't infer direction of a boxed SubMenu.
        // Consider only accepting an enum of special menu widgets?
        // Then we can pass type information.
        #[inline]
        pub fn right<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl SubMenu<kas::dir::Down> {
        /// Construct a sub-menu, opening downwards
        #[inline]
        pub fn down<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl Self {
        /// Construct a sub-menu
        ///
        /// The sub-menu is opened in the `direction` given (contents are always vertical).
        #[inline]
        pub fn new_with_direction<S: Into<AccelString>>(
            direction: D, label: S, list: Vec<BoxedMenu>
        ) -> Self {
            SubMenu {
                core: Default::default(),
                direction,
                key_nav: true,
                label: Label::new(label.into(), TextClass::MenuLabel),
                mark: Mark::new(MarkStyle::Point(direction.as_direction())),
                list: PopupFrame::new(MenuView::new(list)),
                popup_id: None,
            }
        }

        fn open_menu(&mut self, mgr: &mut EventMgr, set_focus: bool) {
            if self.popup_id.is_none() {
                self.popup_id = mgr.add_popup(kas::Popup {
                    id: self.list.id(),
                    parent: self.id(),
                    direction: self.direction.as_direction(),
                });
                if set_focus {
                    mgr.next_nav_focus(self, false, true);
                }
            }
        }
        fn close_menu(&mut self, mgr: &mut EventMgr, restore_focus: bool) {
            if let Some(id) = self.popup_id {
                mgr.close_window(id, restore_focus);
            }
        }

        fn handle_dir_key(&mut self, mgr: &mut EventMgr, cmd: Command) -> Response {
            if self.menu_is_open() {
                if let Some(dir) = cmd.as_direction() {
                    if dir.is_vertical() {
                        mgr.next_nav_focus(self, false, true);
                        Response::Used
                    } else if dir == self.direction.as_direction().reversed() {
                        self.close_menu(mgr, true);
                        Response::Used
                    } else {
                        Response::Unused
                    }
                } else if matches!(cmd, Command::Home | Command::End) {
                    mgr.clear_nav_focus();
                    let rev = cmd == Command::End;
                    mgr.next_nav_focus(self, rev, true);
                    Response::Used
                } else {
                    Response::Unused
                }
            } else if Some(self.direction.as_direction()) == cmd.as_direction() {
                self.open_menu(mgr, true);
                Response::Used
            } else {
                Response::Unused
            }
        }
    }

    impl WidgetConfig for Self {
        fn configure_recurse(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
            self.core_data_mut().id = id;
            mgr.add_accel_keys(self.id_ref(), self.label.keys());
            mgr.new_accel_layer(self.id(), true);

            let id = self.id_ref().make_child(widget_index![self.list]);
            self.list.configure_recurse(mgr, id);

            self.configure(mgr);
        }

        fn key_nav(&self) -> bool {
            self.key_nav
        }
    }

    impl kas::Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            layout::Layout::component(&mut self.label)
        }

        fn spatial_nav(&mut self, _: &mut SetRectMgr, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.frame(&*self, FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw.re(), &self.core.id);
            if self.mark.rect.size != Size::ZERO {
                self.mark.draw(draw, &self.core.id);
            }
        }
    }

    impl Handler for Self {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Activate => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, true);
                    }
                    Response::Used
                }
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    Response::Used
                }
                Event::Command(cmd, _) => self.handle_dir_key(mgr, cmd),
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            self.close_menu(mgr, true);
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems> {
            Some(SubItems {
                label: Some(&mut self.label),
                submenu: Some(&mut self.mark),
                ..Default::default()
            })
        }

        fn menu_is_open(&self) -> bool {
            self.popup_id.is_some()
        }

        fn set_menu_path(&mut self, mgr: &mut EventMgr, target: Option<&WidgetId>, set_focus: bool) {
            match target {
                Some(id) if self.is_ancestor_of(id) => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, set_focus);
                    }
                    if !self.eq_id(id) {
                        for i in 0..self.list.len() {
                            self.list[i].set_menu_path(mgr, target, set_focus);
                        }
                    }
                }
                _ if self.popup_id.is_some() => {
                    self.close_menu(mgr, set_focus);
                }
                _ => (),
            }
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.label.as_str()
        }
    }
}

const MENU_VIEW_COLS: u32 = 5;
const fn menu_view_row_info(row: u32) -> layout::GridChildInfo {
    layout::GridChildInfo {
        col: 0,
        col_end: MENU_VIEW_COLS,
        row,
        row_end: row + 1,
    }
}

impl_scope! {
    /// A menu view
    #[autoimpl(Debug)]
    #[widget]
    struct MenuView<W: Menu> {
        #[widget_core]
        core: CoreData,
        dim: layout::GridDimensions,
        store: layout::DynGridStorage, //NOTE(opt): number of columns is fixed
        list: Vec<W>,
    }

    impl kas::WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.list.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.list.get(index).map(|w| w.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.list.get_mut(index).map(|w| w.as_widget_mut())
        }
    }

    impl kas::Layout for Self {
        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            self.dim = layout::GridDimensions {
                cols: MENU_VIEW_COLS,
                col_spans: self.list.iter_mut().filter_map(|w| w.sub_items().is_none().then(|| ())).count().cast(),
                rows: self.list.len().cast(),
                row_spans: 0,
            };

            let store = &mut self.store;
            let mut solver = layout::GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, store);

            let frame_rules = mgr.frame(FrameStyle::MenuEntry, axis);
            let is_horiz = axis.is_horizontal();
            let with_frame_rules = |rules| if is_horiz {
                frame_rules.surround_as_margin(rules).0
            } else {
                frame_rules.surround_no_margin(rules).0
            };

            for (row, child) in self.list.iter_mut().enumerate() {
                let row = u32::conv(row);
                if let Some(items) = child.sub_items() {
                    if let Some(w) = items.toggle {
                        let info = layout::GridChildInfo::new(0, row);
                        solver.for_child(store, info, |axis| with_frame_rules(w.size_rules(mgr.re(), axis)));
                    }
                    if let Some(w) = items.icon {
                        let info = layout::GridChildInfo::new(1, row);
                        solver.for_child(store, info, |axis| with_frame_rules(w.size_rules(mgr.re(), axis)));
                    }
                    if let Some(w) = items.label {
                        let info = layout::GridChildInfo::new(2, row);
                        solver.for_child(store, info, |axis| with_frame_rules(w.size_rules(mgr.re(), axis)));
                    }
                    if let Some(w) = items.label2 {
                        let info = layout::GridChildInfo::new(3, row);
                        solver.for_child(store, info, |axis| with_frame_rules(w.size_rules(mgr.re(), axis)));
                    }
                    if let Some(w) = items.submenu {
                        let info = layout::GridChildInfo::new(4, row);
                        solver.for_child(store, info, |axis| with_frame_rules(w.size_rules(mgr.re(), axis)));
                    }
                } else {
                    let info = menu_view_row_info(row);
                    solver.for_child(store, info, |axis| child.size_rules(mgr.re(), axis));
                }
            }
            solver.finish(store)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            self.core.rect = rect;
            let store = &mut self.store;
            let mut setter = layout::GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, align, store);

            // Assumption: frame inner margin is at least as large as content margins
            let dir = Direction::Right; // assumption: horiz and vert are the same
            let frame_rules = mgr.size_mgr().frame(FrameStyle::MenuEntry, dir);
            let (_, frame_offset, frame_size) = frame_rules.surround_no_margin(SizeRules::EMPTY);
            let subtract_frame = |mut rect: Rect| {
                rect.pos += Offset::splat(frame_offset);
                rect.size -= Size::splat(frame_size);
                rect
            };

            for (row, child) in self.list.iter_mut().enumerate() {
                let row = u32::conv(row);
                let child_rect = setter.child_rect(store, menu_view_row_info(row));

                if let Some(items) = child.sub_items() {
                    if let Some(w) = items.toggle {
                        let info = layout::GridChildInfo::new(0, row);
                        w.set_rect(mgr, subtract_frame(setter.child_rect(store, info)), align);
                    }
                    if let Some(w) = items.icon {
                        let info = layout::GridChildInfo::new(1, row);
                        w.set_rect(mgr, subtract_frame(setter.child_rect(store, info)), align);
                    }
                    if let Some(w) =  items.label {
                        let info = layout::GridChildInfo::new(2, row);
                        w.set_rect(mgr, subtract_frame(setter.child_rect(store, info)), align);
                    }
                    if let Some(w) = items.label2 {
                        let info = layout::GridChildInfo::new(3, row);
                        w.set_rect(mgr, subtract_frame(setter.child_rect(store, info)), align);
                    }
                    if let Some(w) = items.submenu {
                        let info = layout::GridChildInfo::new(4, row);
                        w.set_rect(mgr, subtract_frame(setter.child_rect(store, info)), align);
                    }

                    child.core_data_mut().rect = child_rect;
                } else {
                    child.set_rect(mgr, child_rect, align);
                }
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            for child in self.list.iter_mut() {
                if let Some(id) = child.find_id(coord) {
                    return Some(id);
                }
            }
            Some(self.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            for child in self.list.iter_mut() {
                child.draw(draw.re());
            }
        }
    }

    impl Self {
        /// Construct from a list of menu items
        pub fn new(list: Vec<W>) -> Self {
            MenuView {
                core: Default::default(),
                dim: Default::default(),
                store: Default::default(),
                list,
            }
        }

        /// Number of menu items
        pub fn len(&self) -> usize {
            self.list.len()
        }
    }

    impl std::ops::Index<usize> for Self {
        type Output = W;

        fn index(&self, index: usize) -> &Self::Output {
            &self.list[index]
        }
    }

    impl std::ops::IndexMut<usize> for Self {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.list[index]
        }
    }
}
