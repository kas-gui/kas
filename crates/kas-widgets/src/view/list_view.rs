// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view widget

use super::{driver, Driver, PressPhase, SelectionError, SelectionMode, SelectionMsg};
#[allow(unused)] // doc links
use crate::ScrollBars;
use crate::{Scrollable, SelectMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, CursorIcon, Scroll};
use kas::layout::solve_size_rules;
use kas::prelude::*;
use kas::updatable::ListData;
use linear_map::set::LinearSet;
use log::{debug, trace};
use std::time::Instant;

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

impl_scope! {
    /// List view widget
    ///
    /// This widget supports a view over a list of shared data items.
    ///
    /// The shared data type `T` must support [`ListData`].
    /// One may use [`kas::updatable::SharedRc`]
    /// or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`], with data type
    /// `<T as ListData>::Item`. Several implementations are available in the
    /// [`driver`] module or a custom implementation may be used.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// # Messages
    ///
    /// When a child pushes a message, the [`ListData::handle_message`] method is
    /// called. After calling [`ListData::handle_message`], this widget attempts to
    /// read and handle [`SelectMsg`].
    ///
    /// When selection is enabled and an item is selected or deselected, this
    /// widget emits a [`SelectionMsg`].
    #[derive(Clone, Debug)]
    #[widget]
    pub struct ListView<
        D: Directional,
        T: ListData + 'static,
        V: Driver<T::Item> = driver::Default,
    > {
        #[widget_core]
        core: CoreData,
        frame_offset: Offset,
        frame_size: Size,
        view: V,
        data: T,
        data_ver: u64,
        widgets: Vec<WidgetData<T::Key, V::Widget>>,
        /// The number of widgets in use (cur_len ≤ widgets.len())
        cur_len: u32,
        direction: D,
        align_hints: AlignHints,
        ideal_visible: i32,
        child_size_min: i32,
        child_size_ideal: i32,
        child_inter_margin: i32,
        child_size: Size,
        scroll: ScrollComponent,
        sel_mode: SelectionMode,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<T::Key>,
        press_phase: PressPhase,
        press_target: Option<T::Key>,
    }

    impl Self where D: Default, V: Default {
        /// Construct a new instance
        ///
        /// This constructor is available where the direction is determined by the
        /// type: for `D: Directional + Default`. In other cases, use
        /// [`ListView::new_with_direction`].
        pub fn new(data: T) -> Self {
            Self::new_with_dir_driver(D::default(), <V as Default>::default(), data)
        }
    }
    impl Self where V: Default {
        /// Construct a new instance with explicit direction
        pub fn new_with_direction(direction: D, data: T) -> Self {
            Self::new_with_dir_driver(direction, <V as Default>::default(), data)
        }
    }
    impl Self where D: Default {
        /// Construct a new instance with explicit view
        pub fn new_with_driver(view: V, data: T) -> Self {
            Self::new_with_dir_driver(D::default(), view, data)
        }
    }
    impl<T: ListData + 'static, V: Driver<T::Item>> ListView<Direction, T, V> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            TkAction::SET_SIZE
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and view
        pub fn new_with_dir_driver(direction: D, view: V, data: T) -> Self {
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                view,
                data,
                data_ver: 0,
                widgets: Default::default(),
                cur_len: 0,
                direction,
                align_hints: Default::default(),
                ideal_visible: 5,
                child_size_min: 0,
                child_size_ideal: 0,
                child_inter_margin: 0,
                child_size: Size::ZERO,
                scroll: Default::default(),
                sel_mode: SelectionMode::None,
                selection: Default::default(),
                press_phase: PressPhase::None,
                press_target: None,
            }
        }

        /// Access the stored data
        pub fn data(&self) -> &T {
            &self.data
        }

        /// Mutably access the stored data
        ///
        /// It may be necessary to use [`ListView::update_view`] to update the view of this data.
        pub fn data_mut(&mut self) -> &mut T {
            &mut self.data
        }

        /// Get a copy of the shared value at `key`
        pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
            self.data.get_cloned(key)
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`ListData::update`]). Other widgets sharing this data are notified
        /// of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, key: &T::Key, data: T::Item) {
            self.data.update(mgr, key, data);
        }

        /// Update shared data
        ///
        /// This is purely a convenience method over [`ListView::set_value`].
        /// It does nothing if no value is found at `key`.
        /// It notifies other widgets of updates to the shared data.
        pub fn update_value<F: Fn(T::Item) -> T::Item>(&self, mgr: &mut EventMgr, key: &T::Key, f: F) {
            if let Some(item) = self.get_value(key) {
                self.set_value(mgr, key, f(item));
            }
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.sel_mode
        }
        /// Set the current selection mode
        pub fn set_selection_mode(&mut self, mode: SelectionMode) -> TkAction {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    TkAction::REDRAW
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    TkAction::REDRAW
                }
                _ => TkAction::empty(),
            }
        }
        /// Set the selection mode (inline)
        #[must_use]
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            let _ = self.set_selection_mode(mode);
            self
        }

        /// Read the list of selected entries
        ///
        /// With mode [`SelectionMode::Single`] this may contain zero or one entry;
        /// use `selected_iter().next()` to extract only the first (optional) entry.
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ T::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &T::Key) -> bool {
            self.selection.contains(key)
        }

        /// Clear all selected items
        pub fn clear_selected(&mut self) -> TkAction {
            if self.selection.is_empty() {
                TkAction::empty()
            } else {
                self.selection.clear();
                TkAction::REDRAW
            }
        }

        /// Directly select an item
        ///
        /// Returns `TkAction::REDRAW` if newly selected, `TkAction::empty()` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, key: T::Key) -> Result<TkAction, SelectionError> {
            match self.sel_mode {
                SelectionMode::None => return Err(SelectionError::Disabled),
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            if !self.data.contains_key(&key) {
                return Err(SelectionError::Key);
            }
            match self.selection.insert(key) {
                true => Ok(TkAction::REDRAW),
                false => Ok(TkAction::empty()),
            }
        }

        /// Directly deselect an item
        ///
        /// Returns `TkAction::REDRAW` if deselected, `TkAction::empty()` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, key: &T::Key) -> TkAction {
            match self.selection.remove(key) {
                true => TkAction::REDRAW,
                false => TkAction::empty(),
            }
        }

        /// Manually trigger an update to handle changed data
        pub fn update_view(&mut self, mgr: &mut EventMgr) {
            let data = &self.data;
            self.selection.retain(|key| data.contains_key(key));
            for w in &mut self.widgets {
                w.key = None;
            }
            mgr.set_rect_mgr(|mgr| self.update_widgets(mgr));
            // Force SET_SIZE so that scroll-bar wrappers get updated
            trace!("update_view triggers SET_SIZE");
            *mgr |= TkAction::SET_SIZE;
        }

        /// Get the direction of contents
        pub fn direction(&self) -> Direction {
            self.direction.as_direction()
        }

        /// Set the preferred number of items visible (inline)
        ///
        /// This affects the (ideal) size request and whether children are sized
        /// according to their ideal or minimum size but not the minimum size.
        #[must_use]
        pub fn with_num_visible(mut self, number: i32) -> Self {
            self.ideal_visible = number;
            self
        }

        /// Construct a position solver. Note: this does more work and updates to
        /// self than is necessary in several cases where it is used.
        fn position_solver(&mut self, mgr: &mut SetRectMgr) -> PositionSolver {
            let data_len = self.data.len();
            let data_len32 = i32::conv(data_len);
            let view_size = self.rect().size;
            let mut content_size = view_size;
            let mut skip;
            if self.direction.is_horizontal() {
                skip = Offset(self.child_size.0 + self.child_inter_margin, 0);
                content_size.0 = (skip.0 * data_len32 - self.child_inter_margin).max(0);
            } else {
                skip = Offset(0, self.child_size.1 + self.child_inter_margin);
                content_size.1 = (skip.1 * data_len32 - self.child_inter_margin).max(0);
            }
            *mgr |= self.scroll.set_sizes(view_size, content_size);

            let offset = u64::conv(self.scroll_offset().extract(self.direction));
            // first visible data item, in downward direction:
            let mut first_data = usize::conv(offset / u64::conv(skip.extract(self.direction)));

            // set_rect allocates enough widgets to view a page; we update widget-data allocations
            let cur_len = self.widgets.len().min(data_len - first_data);
            self.cur_len = cur_len.cast();

            let mut pos_start = self.core.rect.pos + self.frame_offset;
            if self.direction.is_reversed() {
                first_data = (data_len - first_data).saturating_sub(cur_len);
                pos_start += skip * i32::conv(data_len - 1);
                skip = skip * -1;
            }

            PositionSolver {
                pos_start,
                skip,
                size: self.child_size,
                first_data,
                cur_len,
            }
        }

        fn update_widgets(&mut self, mgr: &mut SetRectMgr) {
            let time = Instant::now();
            let solver = self.position_solver(mgr);

            let mut action = TkAction::empty();
            for (i, key) in self
                .data
                .iter_vec_from(solver.first_data, solver.cur_len)
                .into_iter()
                .enumerate()
            {
                let i = solver.first_data + i;
                let id = self.data.make_id(self.id_ref(), &key);
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key) {
                    if let Some(item) = self.data.get_cloned(&key) {
                        w.key = Some(key);
                        mgr.configure(id, &mut w.widget);
                        action |= self.view.set(&mut w.widget, item);
                        solve_size_rules(
                            &mut w.widget,
                            mgr.size_mgr(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                        );
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                }
                w.widget.set_rect(mgr, solver.rect(i), self.align_hints);
            }
            *mgr |= action;
            let dur = (Instant::now() - time).as_micros();
            trace!("ListView::update_widgets completed in {}μs", dur);
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            // TODO: maybe we should support a scrollbar on the other axis?
            // We would need to report a fake min-child-size to enable scrolling.
            let item_min = self.child_size_min + self.child_inter_margin;
            let num = i32::conv(self.data.len());
            let min_size = (item_min * num - self.child_inter_margin).max(0);
            (
                self.direction.is_horizontal() && min_size > size.0,
                self.direction.is_vertical() && min_size > size.1,
            )
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.scroll.max_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.scroll.offset()
        }

        #[inline]
        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            *mgr |= self.scroll.set_offset(offset);
            mgr.set_rect_mgr(|mgr| self.update_widgets(mgr));
            self.scroll.offset()
        }
    }

    impl WidgetChildren for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        #[inline]
        fn get_child(&self, index: usize) -> Option<&dyn Widget> {
            self.widgets.get(index).map(|w| w.widget.as_widget())
        }
        #[inline]
        fn get_child_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
            self.widgets
                .get_mut(index)
                .map(|w| w.widget.as_widget_mut())
        }
        fn find_child_index(&self, id: &WidgetId) -> Option<usize> {
            let key = self.data.reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                self.widgets
                    .iter()
                    .enumerate()
                    .filter_map(|(i, w)| (key == w.key).then(|| i))
                    .next()
            } else {
                None
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = size_mgr.inner_margin().extract(axis);
            let frame = kas::layout::FrameRules::new_sym(0, inner_margin, 0);

            // We use a default widget to find the minimum child size:
            let mut rules = self.view.make().size_rules(size_mgr.re(), axis);
            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_min = rules.min_size();
            }

            if self.widgets.len() > 0 {
                let other = axis.other().map(|mut size| {
                    // Use same logic as in set_rect to find per-child size:
                    let other_axis = axis.flipped();
                    size -= self.frame_size.extract(other_axis);
                    if self.direction.is_horizontal() == other_axis.is_horizontal() {
                        size = (size / self.ideal_visible).min(self.child_size_ideal).max(self.child_size_min);
                    }
                    size
                });
                let axis = AxisInfo::new(axis.is_vertical(), other);
                for w in self.widgets.iter_mut() {
                    rules = rules.max(w.widget.size_rules(size_mgr.re(), axis));
                }
            }

            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_ideal = rules.ideal_size();
                let m = rules.margins_i32();
                self.child_inter_margin = m.0.max(m.1).max(inner_margin);
                rules.multiply_with_margin(2, self.ideal_visible);
                rules.set_stretch(rules.stretch().max(Stretch::High));
            }
            let (rules, offset, size) = frame.surround_with_margin(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, mut align: AlignHints) {
            self.core.rect = rect;

            let mut child_size = rect.size - self.frame_size;
            let num = if self.direction.is_horizontal() {
                child_size.0 = (child_size.0 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                let skip = child_size.0 + self.child_inter_margin;
                align.horiz = None;
                (rect.size.0 + skip - 1) / skip + 1
            } else {
                child_size.1 = (child_size.1 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                let skip = child_size.1 + self.child_inter_margin;
                align.vert = None;
                (rect.size.1 + skip - 1) / skip + 1
            };

            self.child_size = child_size;
            self.align_hints = align;

            let data_len = self.data.len();
            let avail_widgets = self.widgets.len();
            let mut req_widgets = usize::conv(num);
            if data_len <= avail_widgets {
                req_widgets = data_len
            } else if avail_widgets < req_widgets {
                debug!("allocating widgets (old len = {}, new = {})", avail_widgets, req_widgets);
                self.widgets.reserve(req_widgets - avail_widgets);
                for _ in avail_widgets..req_widgets {
                    let widget = self.view.make();
                    self.widgets.push(WidgetData { key: None, widget });
                }
            }
            if req_widgets + 64 <= avail_widgets {
                // Free memory (rarely useful?)
                self.widgets.truncate(req_widgets);
            }
            self.update_widgets(mgr);
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let offset = self.scroll_offset();
            draw.with_clip_region(self.core.rect, offset, |mut draw| {
                for child in &mut self.widgets[..self.cur_len.cast()] {
                    draw.recurse(&mut child.widget);
                    if let Some(ref key) = child.key {
                        if self.selection.contains(key) {
                            draw.selection_box(child.widget.rect());
                        }
                    }
                }
            });
        }
    }

    impl Widget for Self {
        fn configure_recurse(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
            self.core_data_mut().id = id;

            // If data is available but not loaded yet, make some widgets for
            // use by size_rules (this allows better sizing). Configure the new
            // widgets (this allows resource loading which may affect size.)
            self.data_ver = self.data.version();
            if self.widgets.len() == 0 && !self.data.is_empty() {
                let items = self.data.iter_vec(self.ideal_visible.cast());
                let len = items.len();
                debug!("allocating {} widgets", len);
                self.widgets.reserve(len);
                for key in items.into_iter() {
                    let id = self.data.make_id(self.id_ref(), &key);
                    let mut widget = self.view.make();
                    mgr.configure(id, &mut widget);
                    if let Some(item) = self.data.get_cloned(&key) {
                        *mgr |= self.view.set(&mut widget, item);
                    }
                    let key = Some(key);
                    self.widgets.push(WidgetData { key, widget });
                }
            }

            self.configure(mgr);
        }

        fn configure(&mut self, mgr: &mut SetRectMgr) {
            self.data.update_on_handles(mgr.ev_state(), self.id_ref());
            mgr.register_nav_fallback(self.id());
        }

        fn spatial_nav(
            &mut self,
            mgr: &mut SetRectMgr,
            reverse: bool,
            from: Option<usize>,
        ) -> Option<usize> {
            if self.cur_len == 0 {
                return None;
            }

            let solver = self.position_solver(mgr);
            let last_data = self.data.len() - 1;
            let data = if let Some(index) = from {
                let data = solver.child_to_data(index);
                if !reverse && data < last_data {
                    data + 1
                } else if reverse && data > 0 {
                    data - 1
                } else {
                    return None;
                }
            } else if !reverse {
                0
            } else {
                last_data
            };

            let (_, action) = self.scroll.focus_rect(solver.rect(data), self.core.rect);
            if !action.is_empty() {
                *mgr |= action;
                self.update_widgets(mgr);
            }

            Some(data % usize::conv(self.cur_len))
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            let coord = coord + self.scroll.offset();
            for child in &mut self.widgets[..self.cur_len.cast()] {
                if let Some(id) = child.widget.find_id(coord) {
                    return Some(id);
                }
            }
            Some(self.id())
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::HandleUpdate { .. } => {
                    let data_ver = self.data.version();
                    if data_ver > self.data_ver {
                        // TODO(opt): use the update payload to indicate which widgets need updating?
                        self.update_view(mgr);
                        self.data_ver = data_ver;
                    }
                    return Response::Used;
                }
                Event::PressStart { source, coord, .. } => {
                    return if source.is_primary() {
                        mgr.grab_press_unique(self.id(), source, coord, None);
                        self.press_phase = PressPhase::Pan;
                        Response::Used
                    } else {
                        Response::Unused
                    };
                }
                Event::PressMove { coord, .. } => {
                    if let PressPhase::Start(start_coord) = self.press_phase {
                        if mgr.config_test_pan_thresh(coord - start_coord) {
                            self.press_phase = PressPhase::Pan;
                        }
                    }
                    match self.press_phase {
                        PressPhase::Pan => {
                            mgr.update_grab_cursor(self.id(), CursorIcon::Grabbing);
                            // fall through to scroll handler
                        }
                        _ => return Response::Used,
                    }
                }
                Event::PressEnd { ref end_id, .. } => {
                    if self.press_phase == PressPhase::Pan {
                        // fall through to scroll handler
                    } else if end_id.is_some() {
                        if let Some(ref key) = self.press_target {
                            if mgr.config().mouse_nav_focus() {
                                for w in &self.widgets {
                                    if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                                        if w.widget.key_nav() {
                                            mgr.set_nav_focus(w.widget.id(), false);
                                        }
                                        break;
                                    }
                                }
                            }

                            match self.sel_mode {
                                SelectionMode::None => (),
                                SelectionMode::Single => {
                                    mgr.redraw(self.id());
                                    self.selection.clear();
                                    self.selection.insert(key.clone());
                                    mgr.push_msg(SelectionMsg::Select(key.clone()));
                                }
                                SelectionMode::Multiple => {
                                    mgr.redraw(self.id());
                                    if self.selection.remove(key) {
                                        mgr.push_msg(SelectionMsg::Deselect(key.clone()));
                                    } else {
                                        self.selection.insert(key.clone());
                                        mgr.push_msg(SelectionMsg::Select(key.clone()));
                                    }
                                }
                            }
                        }
                        return Response::Used;
                    } else {
                        return Response::Used;
                    }
                }
                Event::Command(cmd, _) => {
                    let last = self.data.len().wrapping_sub(1);
                    if last == usize::MAX || !self.widgets[0].widget.key_nav() {
                        return Response::Unused;
                    }

                    let solver = mgr.set_rect_mgr(|mgr| self.position_solver(mgr));
                    let cur = match mgr.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Response::Unused,
                    };
                    let is_vert = self.direction.is_vertical();
                    let len = solver.cur_len;

                    use Command as C;
                    let data = match cmd {
                        C::Home | C::DocHome => Some(0),
                        C::End | C::DocEnd => Some(last),
                        C::Left | C::WordLeft if !is_vert && cur > 0 => Some(cur - 1),
                        C::Up if is_vert && cur > 0 => Some(cur - 1),
                        C::Right | C::WordRight if !is_vert && cur < last => Some(cur + 1),
                        C::Down if is_vert && cur < last => Some(cur + 1),
                        C::PageUp if cur > 0 => Some(cur.saturating_sub(len / 2)),
                        C::PageDown if cur < last => Some((cur + len / 2).min(last)),
                        // TODO: C::ViewUp, ...
                        _ => None,
                    };
                    return if let Some(i_data) = data {
                        // Set nav focus to i_data and update scroll position
                        let (rect, action) = self.scroll.focus_rect(solver.rect(i_data), self.core.rect);
                        if !action.is_empty() {
                            *mgr |= action;
                            mgr.set_rect_mgr(|mgr| self.update_widgets(mgr));
                        }
                        let index = i_data % usize::conv(self.cur_len);
                        mgr.next_nav_focus(&mut self.widgets[index].widget, false, true);
                        mgr.set_scroll(Scroll::Rect(rect));
                        Response::Used
                    } else {
                        Response::Unused
                    };
                }
                _ => (), // fall through to scroll handler
            }

            let (moved, r) = self.scroll.scroll_by_event(mgr, event, self.id(), self.core.rect);
            if moved {
                mgr.set_rect_mgr(|mgr| self.update_widgets(mgr));
            }
            r
        }

        fn handle_unused(&mut self, mgr: &mut EventMgr, index: usize, event: Event) -> Response {
            if let Event::PressStart { source, coord, .. } = event {
                if source.is_primary() {
                    // We request a grab with our ID, hence the
                    // PressMove/PressEnd events are matched in handle_event().
                    mgr.grab_press_unique(self.id(), source, coord, None);
                    self.press_phase = PressPhase::Start(coord);
                    self.press_target = self.widgets[index].key.clone();
                    Response::Used
                } else {
                    Response::Unused
                }
            } else {
                self.handle_event(mgr, event)
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, index: usize) {
            let key = match self.widgets[index].key.clone() {
                Some(k) => k,
                None => return,
            };

            self.data.handle_message(mgr, &key);

            if let Some(SelectMsg) = mgr.try_pop_msg() {
                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        mgr.redraw(self.id());
                        self.selection.clear();
                        self.selection.insert(key.clone());
                        mgr.push_msg(SelectionMsg::Select(key));
                    }
                    SelectionMode::Multiple => {
                        mgr.redraw(self.id());
                        if self.selection.remove(&key) {
                            mgr.push_msg(SelectionMsg::Deselect(key));
                        } else {
                            self.selection.insert(key.clone());
                            mgr.push_msg(SelectionMsg::Select(key));
                        }
                    }
                }
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventMgr, scroll: Scroll) {
            self.scroll.scroll(mgr, self.rect(), scroll);
            mgr.set_rect_mgr(|mgr| self.update_widgets(mgr));
        }
    }
}

struct PositionSolver {
    pos_start: Coord,
    skip: Offset,
    size: Size,
    first_data: usize,
    cur_len: usize,
}

impl PositionSolver {
    /// Map a child index to a data index
    fn child_to_data(&self, index: usize) -> usize {
        let mut data = (self.first_data / self.cur_len) * self.cur_len + index;
        if data < self.first_data {
            data += self.cur_len;
        }
        data
    }

    /// Rect of data item i
    fn rect(&self, i: usize) -> Rect {
        let pos = self.pos_start + self.skip * i32::conv(i);
        Rect::new(pos, self.size)
    }
}
