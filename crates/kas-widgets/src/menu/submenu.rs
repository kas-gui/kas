// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::{BoxedMenu, Menu, SubItems};
use crate::{AccessLabel, Mark};
use kas::event::{Command, FocusSource};
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::{FrameStyle, MarkStyle, TextClass};
use kas::Popup;

#[impl_self]
mod SubMenu {
    /// A sub-menu
    #[widget {
        layout = self.label;
    }]
    pub struct SubMenu<const TOP_LEVEL: bool, Data> {
        core: widget_core!(),
        #[widget(&())]
        label: AccessLabel,
        // mark is not used in layout but may be used by sub_items
        #[widget(&())]
        mark: Mark,
        #[widget]
        popup: Popup<MenuView<BoxedMenu<Data>>>,
    }

    impl Self {
        /// Construct a sub-menu, opening to the right
        pub fn right<S: Into<AccessString>>(label: S, list: Vec<BoxedMenu<Data>>) -> Self {
            SubMenu::new(label, list, Direction::Right)
        }

        /// Construct a sub-menu, opening downwards
        pub fn down<S: Into<AccessString>>(label: S, list: Vec<BoxedMenu<Data>>) -> Self {
            SubMenu::new(label, list, Direction::Down)
        }

        /// Construct a sub-menu
        #[inline]
        pub fn new<S: Into<AccessString>>(
            label: S,
            list: Vec<BoxedMenu<Data>>,
            direction: Direction,
        ) -> Self {
            SubMenu {
                core: Default::default(),
                label: AccessLabel::new(label).with_class(TextClass::MenuLabel),
                mark: Mark::new(MarkStyle::Point(direction)),
                popup: Popup::new(MenuView::new(list), direction),
            }
        }

        fn open_menu(&mut self, cx: &mut EventCx, data: &Data, set_focus: bool) {
            if self.popup.open(cx, data, self.id()) {
                if set_focus {
                    cx.next_nav_focus(self.id(), false, FocusSource::Key);
                }
            }
        }

        fn handle_dir_key(&mut self, cx: &mut EventCx, data: &Data, cmd: Command) -> IsUsed {
            if self.menu_is_open() {
                if let Some(dir) = cmd.as_direction() {
                    if dir.is_vertical() {
                        let rev = dir.is_reversed();
                        cx.next_nav_focus(None, rev, FocusSource::Key);
                        Used
                    } else if dir == self.popup.direction().reversed() {
                        self.popup.close(cx);
                        Used
                    } else {
                        Unused
                    }
                } else if matches!(cmd, Command::Home | Command::End) {
                    cx.clear_nav_focus();
                    let rev = cmd == Command::End;
                    cx.next_nav_focus(self.id(), rev, FocusSource::Key);
                    Used
                } else {
                    Unused
                }
            } else if Some(self.popup.direction()) == cmd.as_direction() {
                self.open_menu(cx, data, true);
                Used
            } else {
                Unused
            }
        }

        /// Get text contents
        pub fn as_str(&self) -> &str {
            self.label.as_str()
        }
    }

    impl kas::Layout for Self {
        fn draw(&self, mut draw: DrawCx) {
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw.re());
            if self.mark.rect().size != Size::ZERO {
                self.mark.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        fn nav_next(&self, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Events for Self {
        const NAVIGABLE: bool = !TOP_LEVEL;

        type Data = Data;

        fn handle_event(&mut self, cx: &mut EventCx, data: &Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    self.open_menu(cx, data, true);
                    cx.depress_with_key(self.id(), code);
                    Used
                }
                Event::Command(cmd, _) => self.handle_dir_key(cx, data, cmd),
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &Data) {
            if let Some(kas::messages::Activate(code)) = cx.try_pop() {
                self.popup.open(cx, data, self.id());
                cx.depress_with_key(self.id(), code);
            } else {
                self.popup.close(cx);
            }
        }
    }

    impl Menu for Self {
        fn sub_items(&mut self) -> Option<SubItems<'_>> {
            Some(SubItems {
                label: Some(&mut self.label),
                submenu: Some(&mut self.mark),
                ..Default::default()
            })
        }

        fn menu_is_open(&self) -> bool {
            self.popup.is_open()
        }

        fn set_menu_path(
            &mut self,
            cx: &mut EventCx,
            data: &Data,
            target: Option<&Id>,
            set_focus: bool,
        ) {
            if !self.id_ref().is_valid() {
                return;
            }

            match target {
                Some(id) if self.is_ancestor_of(id) => {
                    self.open_menu(cx, data, set_focus);
                }
                _ => self.popup.close(cx),
            }

            for i in 0..self.popup.inner.len() {
                self.popup.inner[i].set_menu_path(cx, data, target, set_focus);
            }
        }
    }
}

const MENU_VIEW_COLS: u32 = 5;
const fn menu_view_row_info(row: u32) -> layout::GridCellInfo {
    layout::GridCellInfo {
        col: 0,
        col_end: MENU_VIEW_COLS,
        row,
        row_end: row + 1,
    }
}

#[impl_self]
mod MenuView {
    /// A menu view
    #[widget]
    struct MenuView<W: Menu> {
        core: widget_core!(),
        dim: layout::GridDimensions,
        store: layout::DynGridStorage, //NOTE(opt): number of columns is fixed
        list: Vec<W>,
    }

    impl kas::Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            self.dim = layout::GridDimensions {
                cols: MENU_VIEW_COLS,
                col_spans: self
                    .list
                    .iter_mut()
                    .filter_map(|w| w.sub_items().is_none().then_some(()))
                    .count()
                    .cast(),
                rows: self.list.len().cast(),
                row_spans: 0,
            };

            let store = &mut self.store;
            let mut solver = layout::GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, store);

            let frame_rules = sizer.frame(FrameStyle::MenuEntry, axis);

            // Assumption: frame inner margin is at least as large as content margins
            let child_rules = SizeRules::EMPTY;
            let (_, _, frame_size_flipped) = sizer
                .frame(FrameStyle::MenuEntry, axis.flipped())
                .surround(child_rules);

            let child_rules = |sizer: SizeCx, w: &mut dyn Tile, mut axis: AxisInfo| {
                axis.sub_other(frame_size_flipped);
                let rules = w.size_rules(sizer, axis);
                frame_rules.surround(rules).0
            };

            for (row, child) in self.list.iter_mut().enumerate() {
                let row = u32::conv(row);
                let info = menu_view_row_info(row);

                // Note: we are required to call child.size_rules even if sub_items are used
                // Note: axis is not modified by the solver in this case
                let rules = child.size_rules(sizer.re(), axis);

                // Note: if we use sub-items, we are required to call size_rules
                // on these for both axes
                if let Some(items) = child.sub_items() {
                    if let Some(w) = items.toggle {
                        let info = layout::GridCellInfo::new(0, row);
                        solver.for_child(store, info, |axis| child_rules(sizer.re(), w, axis));
                    }
                    if let Some(w) = items.icon {
                        let info = layout::GridCellInfo::new(1, row);
                        solver.for_child(store, info, |axis| child_rules(sizer.re(), w, axis));
                    }
                    if let Some(w) = items.label {
                        let info = layout::GridCellInfo::new(2, row);
                        solver.for_child(store, info, |axis| child_rules(sizer.re(), w, axis));
                    }
                    if let Some(w) = items.label2 {
                        let info = layout::GridCellInfo::new(3, row);
                        solver.for_child(store, info, |axis| child_rules(sizer.re(), w, axis));
                    }
                    if let Some(w) = items.submenu {
                        let info = layout::GridCellInfo::new(4, row);
                        solver.for_child(store, info, |axis| child_rules(sizer.re(), w, axis));
                    }
                } else {
                    solver.for_child(store, info, |_| rules);
                }
            }
            solver.finish(store)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, _: AlignHints) {
            widget_set_rect!(rect);
            let store = &mut self.store;
            let hints = AlignHints::NONE;
            let mut setter = layout::GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, store);

            // Assumption: frame inner margin is at least as large as content margins
            let child_rules = SizeRules::EMPTY;
            let (_, frame_x, frame_w) = cx
                .size_cx()
                .frame(FrameStyle::MenuEntry, Direction::Right)
                .surround(child_rules);
            let (_, frame_y, frame_h) = cx
                .size_cx()
                .frame(FrameStyle::MenuEntry, Direction::Down)
                .surround(child_rules);
            let frame_offset = Offset(frame_x, frame_y);
            let frame_size = Size(frame_w, frame_h);
            let subtract_frame = |mut rect: Rect| {
                rect.pos += frame_offset;
                rect.size -= frame_size;
                rect
            };

            for (row, child) in self.list.iter_mut().enumerate() {
                let row = u32::conv(row);
                let child_rect = setter.child_rect(store, menu_view_row_info(row));
                // Note: we are required to call child.set_rect even if sub_items are used
                child.set_rect(cx, child_rect, hints);

                if let Some(items) = child.sub_items() {
                    if let Some(w) = items.toggle {
                        let info = layout::GridCellInfo::new(0, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)), hints);
                    }
                    if let Some(w) = items.icon {
                        let info = layout::GridCellInfo::new(1, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)), hints);
                    }
                    if let Some(w) = items.label {
                        let info = layout::GridCellInfo::new(2, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)), hints);
                    }
                    if let Some(w) = items.label2 {
                        let info = layout::GridCellInfo::new(3, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)), hints);
                    }
                    if let Some(w) = items.submenu {
                        let info = layout::GridCellInfo::new(4, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)), hints);
                    }
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            for child in self.list.iter() {
                child.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.list.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.list.get(index).map(|w| w.as_tile())
        }

        fn probe(&self, coord: Coord) -> Id {
            for child in self.list.iter() {
                if let Some(id) = child.try_probe(coord) {
                    return id;
                }
            }
            self.id()
        }
    }

    impl kas::Widget for Self {
        type Data = W::Data;

        fn child_node<'n>(&'n mut self, data: &'n W::Data, index: usize) -> Option<Node<'n>> {
            self.list.get_mut(index).map(|w| w.as_node(data))
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
