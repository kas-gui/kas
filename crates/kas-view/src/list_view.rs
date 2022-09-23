// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! List view controller

use super::{driver, Driver, PressPhase, SelectionError, SelectionMode, SelectionMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, CursorIcon, Scroll};
use kas::layout::{solve_size_rules, AlignHints};
#[allow(unused)]
use kas::model::SharedData;
use kas::model::{ListData, SharedDataMut};
use kas::prelude::*;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use kas_widgets::SelectMsg;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::time::Instant;

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

impl_scope! {
    /// List view controller
    ///
    /// This widget supports a view over a list of shared data items.
    ///
    /// The shared data type `T` must support [`ListData`].
    /// One may use `[T]`, `Vec<T>` or a custom shared data type.
    ///
    /// The driver `V` must implement [`Driver`] over `T`.
    /// The default driver is [`driver::View`]; others are available in the
    /// [`driver`] module or [`Driver`] may be implemented directly.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// Optionally, data items may be selected; see [`Self::set_selection_mode`].
    ///
    /// # Messages
    ///
    /// When a view widget pushes a message, [`Driver::on_message`] is called.
    ///
    /// When selection is enabled and an item is selected or deselected, this
    /// widget emits a [`SelectionMsg`].
    #[derive(Clone, Debug)]
    #[widget]
    pub struct ListView<
        D: Directional,
        T: ListData,
        V: Driver<T::Item, T> = driver::View,
    > {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        view: V,
        /// Empty widget used for sizing; this must be stored between horiz and vert size rule
        /// calculations for correct line wrapping/layout.
        default_widget: V::Widget,
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
    impl<T: ListData + 'static, V: Driver<T::Item, T>> ListView<Direction, T, V> {
        /// Set the direction of contents
        pub fn set_direction(&mut self, direction: Direction) -> TkAction {
            self.direction = direction;
            TkAction::SET_SIZE
        }
    }
    impl Self {
        /// Construct a new instance with explicit direction and view
        pub fn new_with_dir_driver(direction: D, view: V, data: T) -> Self {
            let default_widget = view.make();
            ListView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                view,
                default_widget,
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

        /// Borrow a reference to the shared value at `key`
        pub fn borrow_value(&self, key: &T::Key) -> Option<impl Borrow<T::Item> + '_> {
            self.data.borrow(key)
        }

        /// Get a copy of the shared value at `key`
        pub fn get_value(&self, key: &T::Key) -> Option<T::Item> {
            self.data.get_cloned(key)
        }

        /// Set shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::borrow_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn set_value(&self, mgr: &mut EventMgr, key: &T::Key, data: T::Item) where T: SharedDataMut {
            self.data.set(mgr, key, data);
        }

        /// Update shared data
        ///
        /// This method updates the shared data, if supported (see
        /// [`SharedDataMut::with_ref_mut`]). Other widgets sharing this data
        /// are notified of the update, if data is changed.
        pub fn update_value<U>(&self, mgr: &mut EventMgr, key: &T::Key, f: impl FnOnce(&mut T::Item) -> U) -> Option<U> where T: SharedDataMut {
            self.data.with_ref_mut(mgr, key, f)
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.sel_mode
        }
        /// Set the current selection mode
        ///
        /// By default, selection is disabled. If enabled, items may be selected
        /// and deselected via mouse-click/touch or via a view widget emitting
        /// [`SelectMsg`].
        ///
        /// On selection and deselection, a [`SelectionMsg`] message is emitted.
        /// This is not sent to [`Driver::on_message`].
        ///
        /// The driver may trigger selection by emitting [`SelectMsg`] from
        /// [`Driver::on_message`]. The driver is not notified of selection
        /// except via [`SelectMsg`] from view widgets. (TODO: reconsider this.)
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
        ///
        /// See [`Self::set_selection_mode`] documentation.
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
            mgr.config_mgr(|mgr| self.update_widgets(mgr));
            // Force SET_SIZE so that scroll-bar wrappers get updated
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
        fn position_solver(&mut self, mgr: &mut ConfigMgr) -> PositionSolver {
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

        fn update_widgets(&mut self, mgr: &mut ConfigMgr) {
            let time = Instant::now();
            let solver = self.position_solver(mgr);
            let mut action = TkAction::empty();

            let keys = self
                .data
                .iter_from(solver.first_data, solver.cur_len);

            let mut count = 0;
            for (i, key) in keys.enumerate() {
                count += 1;
                let i = solver.first_data + i;
                let id = self.data.make_id(self.id_ref(), &key);
                let w = &mut self.widgets[i % solver.cur_len];
                if w.key.as_ref() != Some(&key) {
                    // TODO(opt): we only need to configure the widget once
                    mgr.configure(id, &mut w.widget);

                    if let Some(item) = self.data.borrow(&key) {
                        action |= self.view.set(&mut w.widget, &key, item.borrow());
                        solve_size_rules(
                            &mut w.widget,
                            mgr.size_mgr(),
                            Some(self.child_size.0),
                            Some(self.child_size.1),
                            self.align_hints.horiz,
                            self.align_hints.vert,
                        );
                        w.key = Some(key);
                    } else {
                        w.key = None; // disables drawing and clicking
                    }
                }
                w.widget.set_rect(mgr, solver.rect(i));
            }

            if count < solver.cur_len {
                log::warn!(
                    "{}: data.iter_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    solver.first_data,
                    solver.cur_len,
                );
            }

            *mgr |= action;
            let dur = (Instant::now() - time).as_micros();
            log::trace!(target: "kas_perf::view::list_view", "update_widgets: {dur}μs");
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            // TODO: maybe we should support a scroll bar on the other axis?
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
            mgr.config_mgr(|mgr| self.update_widgets(mgr));
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
                    .filter_map(|(i, w)| (key == w.key).then_some(i))
                    .next()
            } else {
                None
            }
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, size_mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = size_mgr.inner_margins().extract(axis);
            let frame = kas::layout::FrameRules::new(0, inner_margin, (0, 0));

            let other = axis.other().map(|mut size| {
                // Use same logic as in set_rect to find per-child size:
                let other_axis = axis.flipped();
                size -= self.frame_size.extract(other_axis);
                if self.direction.is_horizontal() == other_axis.is_horizontal() {
                    size = (size / self.ideal_visible).min(self.child_size_ideal).max(self.child_size_min);
                }
                size
            });
            axis = AxisInfo::new(axis.is_vertical(), other, axis.align());

            let mut rules = self.default_widget.size_rules(size_mgr.re(), axis);
            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_min = rules.min_size();
            }

            if self.widgets.len() > 0 {
                for w in self.widgets.iter_mut() {
                    rules = rules.max(w.widget.size_rules(size_mgr.re(), axis));
                }
            }

            if axis.is_vertical() == self.direction.is_vertical() {
                self.child_size_ideal = rules.ideal_size();
                let m = rules.margins();
                self.child_inter_margin = m.0.max(m.1).max(inner_margin.0).max(inner_margin.1).cast();
                rules.multiply_with_margin(2, self.ideal_visible);
                rules.set_stretch(rules.stretch().max(Stretch::High));
            } else {
                rules.set_stretch(rules.stretch().max(Stretch::Low));
            }
            let (rules, offset, size) = frame.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            self.align_hints.set_component(axis, axis.align());
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;

            let mut child_size = rect.size - self.frame_size;
            let num = if self.direction.is_horizontal() {
                child_size.0 = (child_size.0 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                let skip = child_size.0 + self.child_inter_margin;
                (rect.size.0 + skip - 1) / skip + 1
            } else {
                child_size.1 = (child_size.1 / self.ideal_visible)
                    .min(self.child_size_ideal)
                    .max(self.child_size_min);
                let skip = child_size.1 + self.child_inter_margin;
                (rect.size.1 + skip - 1) / skip + 1
            };

            self.child_size = child_size;

            let data_len = self.data.len();
            let avail_widgets = self.widgets.len();
            let mut req_widgets = usize::conv(num);
            if data_len <= avail_widgets {
                req_widgets = data_len
            } else if avail_widgets < req_widgets {
                log::debug!("set_rect: allocating widgets (old len = {}, new = {})", avail_widgets, req_widgets);
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
        fn configure(&mut self, mgr: &mut ConfigMgr) {
            // If data is available but not loaded yet, make some widgets for
            // use by size_rules (this allows better sizing). Configure the new
            // widgets (this allows resource loading which may affect size.)
            self.data_ver = self.data.version();
            if self.widgets.len() == 0 && !self.data.is_empty() {
                let iter = self.data.iter_limit(self.ideal_visible.cast());
                let lbound = iter.size_hint().0;
                log::debug!("configure: allocating {} widgets", lbound);
                self.widgets.reserve(lbound);
                for key in iter {
                    let id = self.data.make_id(self.id_ref(), &key);
                    let mut widget = self.view.make();
                    mgr.configure(id, &mut widget);
                    let key = if let Some(item) = self.data.borrow(&key) {
                        *mgr |= self.view.set(&mut widget, &key, item.borrow());
                        Some(key)
                    } else {
                        None
                    };
                    self.widgets.push(WidgetData { key, widget });
                }
            }

            mgr.register_nav_fallback(self.id());
        }

        fn nav_next(
            &mut self,
            mgr: &mut ConfigMgr,
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

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            match event {
                Event::Update { .. } => {
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
                                for w in &mut self.widgets {
                                    if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                                        mgr.next_nav_focus(&mut w.widget, false, false);
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
                Event::Command(cmd) => {
                    let last = self.data.len().wrapping_sub(1);
                    if last == usize::MAX {
                        return Response::Unused;
                    }

                    let solver = mgr.config_mgr(|mgr| self.position_solver(mgr));
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
                            mgr.config_mgr(|mgr| self.update_widgets(mgr));
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
                mgr.config_mgr(|mgr| self.update_widgets(mgr));
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
            let w = &mut self.widgets[index];
            let key = match w.key.clone() {
                Some(k) => k,
                None => return,
            };

            self.view.on_message(mgr, &mut w.widget, &self.data, &key);

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
            mgr.config_mgr(|mgr| self.update_widgets(mgr));
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
