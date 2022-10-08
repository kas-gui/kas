// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Matrix view controller

use super::{driver, Driver, PressPhase, SelectionError, SelectionMode, SelectionMsg};
use kas::event::components::ScrollComponent;
use kas::event::{Command, CursorIcon, Scroll};
use kas::layout::{solve_size_rules, AlignHints};
#[allow(unused)] use kas::model::SharedData;
use kas::model::{MatrixData, SharedDataMut};
use kas::prelude::*;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use kas_widgets::SelectMsg;
use linear_map::set::LinearSet;
use std::borrow::Borrow;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default)]
struct Dim {
    rows: i32,
    cols: i32,
}

#[derive(Clone, Debug, Default)]
struct WidgetData<K, W> {
    key: Option<K>,
    widget: W,
}

impl_scope! {
    /// Matrix view controller
    ///
    /// This widget supports a view over a matrix of shared data items.
    ///
    /// The shared data type `T` must support [`MatrixData`].
    /// One must use a custom shared data type.
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
    pub struct MatrixView<
        T: MatrixData,
        V: Driver<T::Item, T> = driver::View,
    > {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        driver: V,
        /// Empty widget used for sizing; this must be stored between horiz and vert size rule
        /// calculations for correct line wrapping/layout.
        default_widget: V::Widget,
        data: T,
        data_ver: u64,
        widgets: Vec<WidgetData<T::Key, V::Widget>>,
        align_hints: AlignHints,
        ideal_len: Dim,
        alloc_len: Dim,
        /// The number of widgets in use (cur_len ≤ widgets.len())
        cur_len: u32,
        child_size_min: Size,
        child_size_ideal: Size,
        child_inter_margin: Size,
        child_size: Size,
        scroll: ScrollComponent,
        sel_mode: SelectionMode,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<T::Key>,
        press_phase: PressPhase,
        press_target: Option<T::Key>,
    }

    impl Self where V: Default {
        /// Construct a new instance
        pub fn new(data: T) -> Self {
            Self::new_with_driver(<V as Default>::default(), data)
        }
    }
    impl Self {
        /// Construct a new instance with explicit driver
        pub fn new_with_driver(driver: V, data: T) -> Self {
            let default_widget = driver.make();
            MatrixView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                driver,
                default_widget,
                data,
                data_ver: 0,
                widgets: Default::default(),
                align_hints: Default::default(),
                ideal_len: Dim { cols: 3, rows: 5 },
                alloc_len: Dim::default(),
                cur_len: 0,
                child_size_min: Size::ZERO,
                child_size_ideal: Size::ZERO,
                child_inter_margin: Size::ZERO,
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
        /// It may be necessary to use [`MatrixView::update_view`] to update the view of this data.
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

        /// Set the preferred number of items visible (inline)
        ///
        /// This affects the (ideal) size request and whether children are sized
        /// according to their ideal or minimum size but not the minimum size.
        #[must_use]
        pub fn with_num_visible(mut self, cols: i32, rows: i32) -> Self {
            self.ideal_len = Dim { cols, rows };
            self
        }

        /// Construct a position solver. Note: this does more work and updates to
        /// self than is necessary in several cases where it is used.
        fn position_solver(&mut self, mgr: &mut ConfigMgr) -> PositionSolver {
            let (d_cols, d_rows) = self.data.len();
            let data_len = Size(d_cols.cast(), d_rows.cast());
            let view_size = self.rect().size;
            let skip = self.child_size + self.child_inter_margin;
            let content_size = (skip.cwise_mul(data_len) - self.child_inter_margin).max(Size::ZERO);
            *mgr |= self.scroll.set_sizes(view_size, content_size);

            let offset = self.scroll_offset();
            let first_col = usize::conv(u64::conv(offset.0) / u64::conv(skip.0));
            let first_row = usize::conv(u64::conv(offset.1) / u64::conv(skip.1));
            let col_len = self.alloc_len.cols.cast();
            let row_len = self.alloc_len.rows.cast();
            self.cur_len = u32::conv(col_len * row_len);

            let pos_start = self.core.rect.pos + self.frame_offset;

            PositionSolver {
                pos_start,
                skip,
                size: self.child_size,
                first_col,
                first_row,
                col_len,
                row_len,
            }
        }

        fn update_widgets(&mut self, mgr: &mut ConfigMgr) -> PositionSolver {
            let time = Instant::now();
            let solver = self.position_solver(mgr);

            let cols: Vec<_> = self
                .data
                .col_iter_from(solver.first_col, solver.col_len)
                .collect();
            if cols.len() < solver.col_len {
                log::warn!(
                    "{}: data.col_iter_vec_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    solver.first_col,
                    solver.col_len,
                );
            }

            let row_iter = self
                .data
                .row_iter_from(solver.first_row, solver.row_len);

            let mut action = TkAction::empty();
            let mut row_count = 0;
            for (rn, row) in row_iter.enumerate() {
                row_count += 1;
                let ri = solver.first_row + rn;
                for (cn, col) in cols.iter().enumerate() {
                    let ci = solver.first_col + cn;
                    let i = solver.data_to_child(ci, ri);
                    let key = T::make_key(col, &row);
                    let id = self.data.make_id(self.id_ref(), &key);
                    let w = &mut self.widgets[i];
                    if w.key.as_ref() != Some(&key) {
                        mgr.configure(id, &mut w.widget);
                        if let Some(item) = self.data.borrow(&key) {
                            action |= self.driver.set(&mut w.widget, &key, item.borrow());
                            w.key = Some(key);
                            solve_size_rules(
                                &mut w.widget,
                                mgr.size_mgr(),
                                Some(self.child_size.0),
                                Some(self.child_size.1),
                                self.align_hints.horiz,
                                self.align_hints.vert,
                            );
                        } else {
                            w.key = None; // disables drawing and clicking
                        }
                    }
                    w.widget.set_rect(mgr, solver.rect(ci, ri));
                }
            }

            if row_count < solver.row_len {
                log::warn!(
                    "{}: data.row_iter_vec_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    solver.first_row,
                    solver.row_len,
                );
            }

            *mgr |= action;
            let dur = (Instant::now() - time).as_micros();
            log::trace!(target: "kas_perf::view::matrix_view", "update_widgets: {dur}μs");
            solver
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let avail = size - self.frame_size;
            let m = self.child_inter_margin;
            let child_size = Size(avail.0 / self.ideal_len.cols, avail.1 / self.ideal_len.rows)
                .min(self.child_size_ideal).max(self.child_size_min);
            let (d_cols, d_rows) = self.data.len();
            let data_len = Size(d_cols.cast(), d_rows.cast());
            let content_size = ((child_size + m).cwise_mul(data_len) - m).max(Size::ZERO);
            (content_size.0 > size.0, content_size.1 > size.1)
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
                let div = Size(self.ideal_len.cols, self.ideal_len.rows).extract(other_axis);
                (size / div)
                    .min(self.child_size_ideal.extract(other_axis))
                    .max(self.child_size_min.extract(other_axis))
            });
            axis = AxisInfo::new(axis.is_vertical(), other, axis.align());

            let mut rules = self.default_widget.size_rules(size_mgr.re(), axis);
            self.child_size_min.set_component(axis, rules.min_size());

            if !self.widgets.is_empty() {
                for w in self.widgets.iter_mut() {
                    rules = rules.max(w.widget.size_rules(size_mgr.re(), axis));
                }
            }

            self.child_size_ideal.set_component(axis, rules.ideal_size());
            let m = rules.margins();
            self.child_inter_margin
                .set_component(axis, m.0.max(m.1).max(inner_margin.0).max(inner_margin.1).cast());

            let ideal_len = match axis.is_vertical() {
                false => self.ideal_len.cols,
                true => self.ideal_len.rows,
            };
            rules.multiply_with_margin(2, ideal_len);
            rules.set_stretch(rules.stretch().max(Stretch::High));

            let (rules, offset, size) = frame.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            self.align_hints.set_component(axis, axis.align());
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;

            let avail = rect.size - self.frame_size;
            let child_size = Size(avail.0 / self.ideal_len.cols, avail.1 / self.ideal_len.rows)
                .min(self.child_size_ideal).max(self.child_size_min);
            self.child_size = child_size;

            let (d_cols, d_rows) = self.data.len();
            let data_len = Size(d_cols.cast(), d_rows.cast());
            let (avail_widgets, mut req_widgets) = (self.widgets.len(), d_cols * d_rows);
            let vis_len;
            if avail_widgets >= req_widgets {
                // Case: enough children to allocate all data directly
                vis_len = data_len;
            } else {
                // Case: reallocate children when scrolling
                let skip = child_size + self.child_inter_margin;
                vis_len = data_len.min(
                    (rect.size + skip - Size::splat(1)).cwise_div(skip) + Size::splat(1)
                );
                req_widgets = usize::conv(vis_len.0) * usize::conv(vis_len.1);
            }
            self.alloc_len = Dim {
                cols: vis_len.0,
                rows: vis_len.1,
            };

            if avail_widgets < req_widgets {
                log::debug!("set_rect: allocating widgets (old len = {}, new = {})", avail_widgets, req_widgets);
                self.widgets.reserve(req_widgets - avail_widgets);
                for _ in avail_widgets..req_widgets {
                    let widget = self.driver.make();
                    self.widgets.push(WidgetData { key: None, widget });
                }
            } else if req_widgets + 64 <= avail_widgets {
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
            let num = self.cur_len.cast();
            for child in &mut self.widgets[..num] {
                if child.key.is_some() {
                    if let Some(id) = child.widget.find_id(coord) {
                        return Some(id);
                    }
                }
            }
            Some(self.id())
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let offset = self.scroll_offset();
            let rect = self.rect() + offset;
            let num = self.cur_len.cast();
            draw.with_clip_region(self.core.rect, offset, |mut draw| {
                for child in &mut self.widgets[..num] {
                    // Note: we don't know which widgets within 0..num are
                    // visible, so check intersection before drawing:
                    if rect.intersection(&child.widget.rect()).is_some() {
                        if let Some(ref key) = child.key {
                            draw.recurse(&mut child.widget);
                            if self.selection.contains(key) {
                                draw.selection_box(child.widget.rect());
                            }
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
            if self.widgets.is_empty() && !self.data.is_empty() {
                let cols: Vec<_> = self.data.col_iter_limit(self.ideal_len.cols.cast()).collect();
                let rows = self.data.row_iter_limit(self.ideal_len.rows.cast());
                let lbound = cols.len() * rows.size_hint().0;
                log::debug!("configure: allocating {} widgets", lbound);
                self.widgets.reserve(lbound);
                for row in rows {
                    for col in cols.iter() {
                        let key = T::make_key(col, &row);
                        let id = self.data.make_id(self.id_ref(), &key);
                        let mut widget = self.driver.make();
                        mgr.configure(id, &mut widget);
                        let key = if let Some(item) = self.data.borrow(&key) {
                            *mgr |= self.driver.set(&mut widget, &key, item.borrow());
                            Some(key)
                        } else {
                            None
                        };
                        self.widgets.push(WidgetData { key, widget });
                    }
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

            let mut solver = self.position_solver(mgr);
            let (d_cols, d_rows) = self.data.len();
            let (ci, ri) = if let Some(index) = from {
                let (ci, ri) = solver.child_to_data(index);
                if !reverse {
                    if ci + 1 < d_cols{
                        (ci + 1, ri)
                    } else if ri + 1 < d_rows {
                        (0, ri + 1)
                    } else {
                        return None;
                    }
                } else {
                    if ci > 0 {
                        (ci - 1, ri)
                    } else if ri > 0 {
                        (d_cols - 1, ri - 1)
                    } else {
                        return None;
                    }
                }
            } else if !reverse {
                (0, 0)
            } else {
                (d_cols - 1, d_rows - 1)
            };

            let (_, action) = self.scroll.focus_rect(solver.rect(ci, ri), self.core.rect);
            if !action.is_empty() {
                *mgr |= action;
                solver = self.update_widgets(mgr);
            }

            Some(solver.data_to_child(ci, ri))
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
                    if self.data.is_empty() {
                        return Response::Unused;
                    }
                    let (d_cols, d_rows) = self.data.len();
                    let (last_col, last_row) = (d_cols.wrapping_sub(1), d_rows.wrapping_sub(1));

                    let mut solver = mgr.config_mgr(|mgr| self.position_solver(mgr));
                    let (ci, ri) = match mgr.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Response::Unused,
                    };

                    use Command as C;
                    let data = match cmd {
                        C::DocHome => Some((0, 0)),
                        C::DocEnd => Some((last_col, last_row)),
                        C::Home => Some((0, ri)),
                        C::End => Some((last_col, ri)),
                        C::Left | C::WordLeft if ci > 0 => Some((ci - 1, ri)),
                        C::Up if ri > 0 => Some((ci, ri - 1)),
                        C::Right | C::WordRight if ci < last_col => Some((ci + 1, ri)),
                        C::Down if ri < last_row => Some((ci, ri + 1)),
                        C::PageUp if ri > 0 => {
                            Some((ci, ri.saturating_sub(solver.row_len / 2)))
                        }
                        C::PageDown if ri < last_row => {
                            Some((ci, (ri + solver.row_len / 2).min(last_row)))
                        }
                        // TODO: C::ViewUp, ...
                        _ => None,
                    };
                    return if let Some((ci, ri)) = data {
                        // Set nav focus and update scroll position
                        let (rect, action) = self.scroll.focus_rect(solver.rect(ci, ri), self.core.rect);
                        if !action.is_empty() {
                            *mgr |= action;
                            solver = mgr.config_mgr(|mgr| self.update_widgets(mgr));
                        }

                        let index = solver.data_to_child(ci, ri);
                        #[cfg(debug_assertions)] {
                            let rk = self.data
                                .row_iter_from(ri, 1)
                                .next()
                                .expect("data row len > data.row_iter_vec len");
                            let ck = self.data
                                .col_iter_from(ci, 1)
                                .next()
                                .expect("data col len > data.col_iter_vec len");
                            let key = T::make_key(&ck, &rk);
                            assert_eq!(
                                self.widgets[index].widget.id(),
                                self.data.make_id(self.id_ref(), &key),
                            );
                        }

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

            self.driver.on_message(mgr, &mut w.widget, &self.data, &key);

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
    skip: Size,
    size: Size,
    first_col: usize,
    first_row: usize,
    col_len: usize,
    row_len: usize,
}

impl PositionSolver {
    /// Map a data index to child index
    fn data_to_child(&self, ci: usize, ri: usize) -> usize {
        (ci % self.col_len) + (ri % self.row_len) * self.col_len
    }

    /// Map a child index to `(col_index, row_index)`
    fn child_to_data(&self, index: usize) -> (usize, usize) {
        let col_start = (self.first_col / self.col_len) * self.col_len;
        let row_start = (self.first_row / self.row_len) * self.row_len;
        let mut col_index = col_start + index % self.col_len;
        let mut row_index = row_start + index / self.col_len;
        if col_index < self.first_col {
            col_index += self.col_len;
        }
        if row_index < self.first_row {
            row_index += self.row_len;
        }
        (col_index, row_index)
    }

    /// Rect of data item (ci, ri)
    fn rect(&self, ci: usize, ri: usize) -> Rect {
        let pos = self.pos_start + self.skip.cwise_mul(Size(ci.cast(), ri.cast()));
        Rect::new(pos, self.size)
    }
}
