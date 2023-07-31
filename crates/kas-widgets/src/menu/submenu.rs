// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Sub-menu

use super::{BoxedMenu, Menu, SubItems};
use crate::{AccelLabel, Mark, PopupFrame};
use kas::event::{Command, Scroll};
use kas::layout::{self, RulesSetter, RulesSolver};
use kas::prelude::*;
use kas::theme::{FrameStyle, MarkStyle, TextClass};
use kas::WindowId;

impl_scope! {
    /// A sub-menu
    #[widget {
        layout = self.label;
    }]
    pub struct SubMenu<Data, D: Directional> {
        core: widget_core!(),
        direction: D,
        pub(crate) navigable: bool,
        #[widget(&())]
        label: AccelLabel,
        #[widget(&())]
        mark: Mark,
        #[widget]
        list: PopupFrame<MenuView<BoxedMenu<Data>>>,
        popup_id: Option<WindowId>,
    }

    impl Self
    where
        D: Default,
    {
        /// Construct a sub-menu
        #[inline]
        pub fn new<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu<Data>>) -> Self {
            SubMenu::new_with_direction(Default::default(), label, list)
        }
    }

    impl<Data> SubMenu<Data, kas::dir::Right> {
        /// Construct a sub-menu, opening to the right
        // NOTE: this is used since we can't infer direction of a boxed SubMenu.
        // Consider only accepting an enum of special menu widgets?
        // Then we can pass type information.
        #[inline]
        pub fn right<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu<Data>>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl<Data> SubMenu<Data, kas::dir::Down> {
        /// Construct a sub-menu, opening downwards
        #[inline]
        pub fn down<S: Into<AccelString>>(label: S, list: Vec<BoxedMenu<Data>>) -> Self {
            SubMenu::new(label, list)
        }
    }

    impl Self {
        /// Construct a sub-menu
        ///
        /// The sub-menu is opened in the `direction` given (contents are always vertical).
        #[inline]
        pub fn new_with_direction<S: Into<AccelString>>(
            direction: D,
            label: S,
            list: Vec<BoxedMenu<Data>>,
        ) -> Self {
            SubMenu {
                core: Default::default(),
                direction,
                navigable: true,
                label: AccelLabel::new(label).with_class(TextClass::MenuLabel),
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
                    mgr.next_nav_focus(self.id(), false, true);
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
                        let rev = dir.is_reversed();
                        mgr.next_nav_focus(self.id(), rev, true);
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
                    mgr.next_nav_focus(self.id(), rev, true);
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

    impl kas::Layout for Self {
        fn nav_next(&self, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            self.rect().contains(coord).then(|| self.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            draw.frame(self.rect(), FrameStyle::MenuEntry, Default::default());
            self.label.draw(draw.re_id(self.id()));
            if self.mark.rect().size != Size::ZERO {
                draw.recurse(&mut self.mark);
            }
        }
    }

    impl Events for Self {
        type Data = Data;

        fn pre_configure(&mut self, cx: &mut ConfigCx, id: WidgetId) {
            self.core.id = id;
            // FIXME: new layer should apply to self.list but not to self.label.
            // We don't currently have a way to do that. Possibly we should
            // remove `EventMgr::add_accel_keys` bindings, simply checking all
            // visible widgets whenever a shortcut key is pressed (also related:
            // currently all pages of a TabStack have active shortcut keys).
            cx.new_accel_layer(self.id(), true);
        }

        fn navigable(&self) -> bool {
            self.navigable
        }

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Command(cmd) if cmd.is_activate() => {
                    if self.popup_id.is_none() {
                        self.open_menu(mgr, true);
                    }
                    Response::Used
                }
                Event::Command(cmd) => self.handle_dir_key(mgr, cmd),
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, _: &Self::Data, mgr: &mut EventMgr) {
            if let Some(kas::message::Activate) = mgr.try_pop() {
                if self.popup_id.is_none() {
                    self.open_menu(mgr, true);
                }
            } else {
                self.close_menu(mgr, true);
            }
        }

        fn handle_scroll(&mut self, _: &Self::Data, mgr: &mut EventMgr, _: Scroll) {
            mgr.set_scroll(Scroll::None);
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

        fn set_menu_path(
            &mut self,
            mgr: &mut EventMgr,
            target: Option<&WidgetId>,
            set_focus: bool,
        ) {
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
            self.label.get_str()
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
    #[widget]
    struct MenuView<W: Menu> {
        core: widget_core!(),
        dim: layout::GridDimensions,
        store: layout::DynGridStorage, //NOTE(opt): number of columns is fixed
        list: Vec<W>,
    }

    impl kas::Widget for Self {
        type Data = W::Data;

        fn for_child_node(
            &mut self,
            data: &W::Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.list.get_mut(index) {
                closure(w.as_node(data));
            }
        }
    }

    impl kas::Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.list.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.list.get(index).map(|w| w.as_layout())
        }

        fn size_rules(&mut self, mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
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

            let frame_rules = mgr.frame(FrameStyle::MenuEntry, axis);
            let is_horiz = axis.is_horizontal();

            // Assumption: frame inner margin is at least as large as content margins
            let child_rules = SizeRules::EMPTY;
            let (_, _, frame_size_flipped) = mgr
                .frame(FrameStyle::MenuEntry, axis.flipped())
                .surround(child_rules);

            let child_rules = |mgr: SizeMgr, w: &mut dyn Layout, mut axis: AxisInfo| {
                axis.sub_other(frame_size_flipped);
                let rules = w.size_rules(mgr, axis);
                frame_rules.surround(rules).0
            };

            for (row, child) in self.list.iter_mut().enumerate() {
                let row = u32::conv(row);
                let info = menu_view_row_info(row);
                if is_horiz {
                    // Note: we are required to call child.size_rules even if sub_items are used
                    // Note: axis is not modified by the solver in this case
                    let row_rules = child.size_rules(mgr.re(), axis);

                    if let Some(items) = child.sub_items() {
                        if let Some(w) = items.toggle {
                            let info = layout::GridChildInfo::new(0, row);
                            solver.for_child(store, info, |axis| child_rules(mgr.re(), w, axis));
                        }
                        if let Some(w) = items.icon {
                            let info = layout::GridChildInfo::new(1, row);
                            solver.for_child(store, info, |axis| child_rules(mgr.re(), w, axis));
                        }
                        if let Some(w) = items.label {
                            let info = layout::GridChildInfo::new(2, row);
                            solver.for_child(store, info, |axis| child_rules(mgr.re(), w, axis));
                        }
                        if let Some(w) = items.label2 {
                            let info = layout::GridChildInfo::new(3, row);
                            solver.for_child(store, info, |axis| child_rules(mgr.re(), w, axis));
                        }
                        if let Some(w) = items.submenu {
                            let info = layout::GridChildInfo::new(4, row);
                            solver.for_child(store, info, |axis| child_rules(mgr.re(), w, axis));
                        }
                    } else {
                        solver.for_child(store, info, |_| row_rules);
                    }
                } else {
                    // axis is vertical
                    if child.sub_items().is_some() {
                        solver.for_child(store, info, |axis| child_rules(mgr.re(), child, axis))
                    } else {
                        solver.for_child(store, info, |axis| child.size_rules(mgr.re(), axis))
                    }
                }
            }
            solver.finish(store)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            let store = &mut self.store;
            let mut setter = layout::GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, store);

            // Assumption: frame inner margin is at least as large as content margins
            let child_rules = SizeRules::EMPTY;
            let (_, frame_x, frame_w) = cx
                .size_mgr()
                .frame(FrameStyle::MenuEntry, Direction::Right)
                .surround(child_rules);
            let (_, frame_y, frame_h) = cx
                .size_mgr()
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
                child.set_rect(cx, child_rect);

                if let Some(items) = child.sub_items() {
                    if let Some(w) = items.toggle {
                        let info = layout::GridChildInfo::new(0, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)));
                    }
                    if let Some(w) = items.icon {
                        let info = layout::GridChildInfo::new(1, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)));
                    }
                    if let Some(w) = items.label {
                        let info = layout::GridChildInfo::new(2, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)));
                    }
                    if let Some(w) = items.label2 {
                        let info = layout::GridChildInfo::new(3, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)));
                    }
                    if let Some(w) = items.submenu {
                        let info = layout::GridChildInfo::new(4, row);
                        w.set_rect(cx, subtract_frame(setter.child_rect(store, info)));
                    }
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
                draw.recurse(child);
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
