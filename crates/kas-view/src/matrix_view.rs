// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Matrix view controller

use super::*;
use kas::event::components::ScrollComponent;
use kas::event::{Command, FocusSource, Scroll};
use kas::layout::{solve_size_rules, AlignHints};
use kas::prelude::*;
use kas::theme::SelectionStyle;
use kas::NavAdvance;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
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
    /// View controller for 2D indexable data (matrix)
    ///
    /// This widget generates a view over a list of data items via the
    /// [`MatrixData`] trait. "View widgets" are constructed via a [`Driver`] to
    /// represent visible data items. These view widgets are reassigned as
    /// required when the matrix is scrolled, keeping the number of widgets in
    /// use roughly proportional to the number of data items within the view.
    ///
    /// Each view widget has an [`Id`] corresponding to its current data
    /// item, and may handle events and emit messages like other widegts.
    /// See [`Driver`] documentation for more on event handling.
    ///
    /// This widget is [`Scrollable`], supporting keyboard, wheel and drag
    /// scrolling. You may wish to wrap this widget with [`ScrollBars`].
    ///
    /// Optionally, data items may be selected; see [`Self::set_selection_mode`].
    /// If enabled, [`SelectionMsg`] messages are reported; view widgets may
    /// emit [`kas::messages::Select`] to have themselves be selected.
    #[derive(Clone, Debug)]
    #[widget]
    pub struct MatrixView<A: MatrixData, V: Driver<A::Item, A>> {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        driver: V,
        widgets: Vec<WidgetData<A::Key, V::Widget>>,
        align_hints: AlignHints,
        ideal_len: Dim,
        alloc_len: Dim,
        data_len: Size,
        /// The number of (cols, rows) in use
        cur_len: (u32, u32),
        first_data: (u32, u32),
        child_size_min: Size,
        child_size_ideal: Size,
        child_inter_margin: Size,
        child_size: Size,
        scroll: ScrollComponent,
        sel_mode: SelectionMode,
        sel_style: SelectionStyle,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<A::Key>,
        press_target: Option<(usize, A::Key)>,
    }

    impl Self {
        /// Construct a new instance
        pub fn new(driver: V) -> Self {
            MatrixView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                driver,
                widgets: Default::default(),
                align_hints: Default::default(),
                ideal_len: Dim { cols: 3, rows: 5 },
                alloc_len: Dim::default(),
                data_len: Size::ZERO,
                cur_len: (0, 0),
                first_data: (0, 0),
                child_size_min: Size::ZERO,
                child_size_ideal: Size::ZERO,
                child_inter_margin: Size::ZERO,
                child_size: Size::ZERO,
                scroll: Default::default(),
                sel_mode: SelectionMode::None,
                sel_style: SelectionStyle::Highlight,
                selection: Default::default(),
                press_target: None,
            }
        }

        /// Get the current selection mode
        pub fn selection_mode(&self) -> SelectionMode {
            self.sel_mode
        }
        /// Set the current selection mode
        ///
        /// By default, selection is disabled. If enabled, items may be selected
        /// and deselected via mouse-click/touch or via a view widget emitting
        /// [`Select`].
        ///
        /// On selection and deselection, a [`SelectionMsg`] message is emitted.
        /// This is not sent to [`Driver::on_messages`].
        ///
        /// The driver may trigger selection by emitting [`Select`] from
        /// [`Driver::on_messages`]. The driver is not notified of selection
        /// except via [`Select`] from view widgets. (TODO: reconsider this.)
        ///
        /// [`Select`]: kas::messages::Select
        pub fn set_selection_mode(&mut self, mode: SelectionMode) -> Action {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    Action::REDRAW
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    Action::REDRAW
                }
                _ => Action::empty(),
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

        /// Get the current selection style
        pub fn selection_style(&self) -> SelectionStyle {
            self.sel_style
        }
        /// Set the current selection style
        ///
        /// By default, [`SelectionStyle::Highlight`] is used. Other modes may
        /// add margin between elements.
        pub fn set_selection_style(&mut self, style: SelectionStyle) -> Action {
            let action = if style.is_external() != self.sel_style.is_external() {
                Action::RESIZE
            } else {
                Action::empty()
            };
            self.sel_style = style;
            action
        }
        /// Set the selection style (inline)
        ///
        /// See [`Self::set_selection_style`] documentation.
        #[must_use]
        pub fn with_selection_style(mut self, style: SelectionStyle) -> Self {
            self.sel_style = style;
            self
        }

        /// Read the list of selected entries
        ///
        /// With mode [`SelectionMode::Single`] this may contain zero or one entry;
        /// use `selected_iter().next()` to extract only the first (optional) entry.
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ A::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &A::Key) -> bool {
            self.selection.contains(key)
        }

        /// Clear all selected items
        pub fn clear_selected(&mut self) -> Action {
            if self.selection.is_empty() {
                Action::empty()
            } else {
                self.selection.clear();
                Action::REDRAW
            }
        }

        /// Directly select an item
        ///
        /// Does nothing if [`Self::selection_mode`] is [`SelectionMode::None`].
        /// Does not verify the validity of `key`.
        /// Does not send [`SelectionMsg`] messages.
        ///
        /// Returns `Action::REDRAW` if newly selected, `Action::empty()` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, key: A::Key) -> Action {
            match self.sel_mode {
                SelectionMode::None => return Action::empty(),
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            match self.selection.insert(key) {
                true => Action::REDRAW,
                false => Action::empty(),
            }
        }

        /// Directly deselect an item
        ///
        /// Returns `Action::REDRAW` if deselected, `Action::empty()` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, key: &A::Key) -> Action {
            match self.selection.remove(key) {
                true => Action::REDRAW,
                false => Action::empty(),
            }
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

        fn position_solver(&self) -> PositionSolver {
            PositionSolver {
                pos_start: self.core.rect.pos + self.frame_offset,
                skip: self.child_size + self.child_inter_margin,
                size: self.child_size,
                first_data: self.first_data,
                cur_len: self.cur_len,
            }
        }

        fn update_widgets(&mut self, cx: &mut ConfigCx, data: &A) -> PositionSolver {
            let time = Instant::now();

            let offset = self.scroll_offset();
            let skip = (self.child_size + self.child_inter_margin).max(Size(1, 1));
            let data_len = data.len();
            let col_len = data_len.0.min(self.alloc_len.cols.cast());
            let row_len = data_len.1.min(self.alloc_len.rows.cast());
            let first_col = usize::conv(u64::conv(offset.0) / u64::conv(skip.0))
                .min(data_len.0 - col_len);
            let first_row = usize::conv(u64::conv(offset.1) / u64::conv(skip.1))
                .min(data_len.1 - row_len);
            self.cur_len = (col_len.cast(), row_len.cast());
            debug_assert!(self.num_children() <= self.widgets.len());
            self.first_data = (first_row.cast(), first_col.cast());

            let solver = self.position_solver();

            let cols: Vec<_> = data
                .col_iter_from(first_col, col_len)
                .collect();
            if cols.len() < col_len {
                log::warn!(
                    "{}: data.col_iter_vec_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    first_col,
                    col_len,
                );
            }

            let row_iter = data.row_iter_from(first_row, row_len);

            let mut row_count = 0;
            for (rn, row) in row_iter.enumerate() {
                row_count += 1;
                let ri = first_row + rn;
                for (cn, col) in cols.iter().enumerate() {
                    let ci = first_col + cn;
                    let i = solver.data_to_child(ci, ri);
                    let key = data.make_key(col, &row);
                    let id = key.make_id(self.id_ref());
                    let w = &mut self.widgets[i];
                    if w.key.as_ref() != Some(&key) {
                        self.driver.set_key(&mut w.widget, &key);

                        if let Some(item) = data.borrow(&key) {
                            cx.configure(w.widget.as_node(item.borrow()), id);

                            w.key = Some(key);
                            solve_size_rules(
                                &mut w.widget,
                                cx.size_cx(),
                                Some(self.child_size.0),
                                Some(self.child_size.1),
                                self.align_hints.horiz,
                                self.align_hints.vert,
                            );
                        } else {
                            w.key = None; // disables drawing and clicking
                        }
                    } else if let Some(item) = data.borrow(&key) {
                        cx.update(w.widget.as_node(item.borrow()));
                    }
                    w.widget.set_rect(cx, solver.rect(ci, ri));
                }
            }

            if row_count < row_len {
                log::warn!(
                    "{}: data.row_iter_vec_from({}, {}) yielded insufficient items (possibly incorrect data.len())", self.identify(),
                    first_row,
                    row_len,
                );
            }

            let dur = (Instant::now() - time).as_micros();
            log::trace!(target: "kas_perf::view::matrix_view", "update_widgets: {dur}μs");
            solver
        }

        fn update_content_size(&mut self, cx: &mut ConfigCx) {
            let view_size = self.rect().size - self.frame_size;
            let skip = self.child_size + self.child_inter_margin;
            let content_size = (skip.cwise_mul(self.data_len) - self.child_inter_margin)
                .max(Size::ZERO);
            let action = self.scroll.set_sizes(view_size, content_size);
            cx.action(self, action);
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let m = self.child_inter_margin;
            let step = self.child_size + m;
            let content_size = (step.cwise_mul(self.data_len) - m).max(Size::ZERO);
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
        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            cx.action(&self, action);
            cx.request_update(self.id(), false);
            self.scroll.offset()
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            usize::conv(self.cur_len.0) * usize::conv(self.cur_len.1)
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.widgets.get(index).map(|w| w.widget.as_layout())
        }
        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let num = self.num_children();
            let key = A::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                self.widgets[0..num]
                    .iter()
                    .enumerate()
                    .filter_map(|(i, w)| (key == w.key).then_some(i))
                    .next()
            } else {
                None
            }
        }

        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            // We use an invisible frame for highlighting selections, drawing into the margin
            let inner_margin = if self.sel_style.is_external() {
                sizer.inner_margins().extract(axis)
            } else {
                (0, 0)
            };
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

            let mut child_size_min = i32::MAX;
            let mut rules = SizeRules::EMPTY;
            for w in self.widgets.iter_mut() {
                if w.key.is_some() {
                    let child_rules = w.widget.size_rules(sizer.re(), axis);
                    child_size_min = child_size_min.min(child_rules.min_size());
                    rules = rules.max(child_rules);
                }
            }
            self.child_size_min.set_component(axis, child_size_min);
            self.child_size_ideal.set_component(axis, rules.ideal_size());

            let m = rules.margins();
            self.child_inter_margin.set_component(
                axis,
                m.0.max(m.1).max(inner_margin.0).max(inner_margin.1).cast(),
            );

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

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;

            // Widgets need configuring and updating: do so by updating self.
            self.cur_len = (0, 0); // hack: prevent drawing in the mean-time
            cx.request_update(self.id(), false);

            let avail = rect.size - self.frame_size;
            let child_size = Size(avail.0 / self.ideal_len.cols, avail.1 / self.ideal_len.rows)
                .min(self.child_size_ideal)
                .max(self.child_size_min);
            self.child_size = child_size;
            self.update_content_size(cx);

            let skip = self.child_size + self.child_inter_margin;
            if skip.0 == 0 || skip.1 == 0 {
                self.alloc_len = Dim { cols: 0, rows: 0 };
                return;
            }
            let vis_len = (rect.size + skip - Size::splat(1)).cwise_div(skip) + Size::splat(1);
            let req_widgets = usize::conv(vis_len.0) * usize::conv(vis_len.1);

            self.alloc_len = Dim {
                cols: vis_len.0,
                rows: vis_len.1,
            };

            let avail_widgets = self.widgets.len();
            if avail_widgets < req_widgets {
                log::debug!(
                    "set_rect: allocating widgets (old len = {}, new = {})",
                    avail_widgets,
                    req_widgets
                );
                self.widgets.resize_with(req_widgets, || {
                    WidgetData {
                        key: None,
                        widget: self.driver.make(&A::Key::default()),
                    }
                });
            } else if req_widgets + 64 <= avail_widgets {
                // Free memory (rarely useful?)
                self.widgets.truncate(req_widgets);
            }
            debug_assert!(self.widgets.len() >= req_widgets);
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn find_id(&mut self, coord: Coord) -> Option<Id> {
            if !self.rect().contains(coord) {
                return None;
            }

            let num = self.num_children();
            let coord = coord + self.scroll.offset();
            for child in &mut self.widgets[..num] {
                if child.key.is_some() {
                    if let Some(id) = child.widget.find_id(coord) {
                        return Some(id);
                    }
                }
            }
            Some(self.id())
        }

        fn draw(&mut self, mut draw: DrawCx) {
            let offset = self.scroll_offset();
            let rect = self.rect() + offset;
            let num = self.num_children();
            draw.with_clip_region(self.core.rect, offset, |mut draw| {
                for child in &mut self.widgets[..num] {
                    if let Some(ref key) = child.key {
                        // Note: we don't know which widgets within 0..num are
                        // visible, so check intersection before drawing:
                        if rect.intersection(&child.widget.rect()).is_some() {
                            if self.selection.contains(key) {
                                draw.selection(child.widget.rect(), self.sel_style);
                            }
                            draw.recurse(&mut child.widget);
                        }
                    }
                }
            });
        }
    }

    impl Events for Self {
        #[inline]
        fn make_child_id(&mut self, _: usize) -> Id {
            // We configure children in update_widgets and do not want this method to be called
            unimplemented!()
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            if self.widgets.is_empty() {
                // Initial configure: ensure some widgets are loaded to allow
                // better sizing of self.
                self.child_size = Size::splat(1); // hack: avoid div by 0

                let len = self.ideal_len.cols * self.ideal_len.rows;
                self.widgets.resize_with(len.cast(), || {
                    WidgetData {
                        key: None,
                        widget: self.driver.make(&A::Key::default()),
                    }
                });
                self.alloc_len = self.ideal_len;
            }

            cx.register_nav_fallback(self.id());
        }

        fn configure_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {}

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            self.selection.retain(|key| data.contains_key(key));

            let (d_cols, d_rows) = data.len();
            let data_len = Size(d_cols.cast(), d_rows.cast());
            if data_len != self.data_len {
                self.data_len = data_len;
                // We must call at least SET_RECT to update scrollable region
                // RESIZE allows recalculation of child widget size which may
                // have been zero if no data was initially available!
                cx.resize(&self);
            }

            self.update_widgets(cx, data);
            self.update_content_size(cx);
        }

        fn update_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {}

        fn handle_event(&mut self, cx: &mut EventCx, data: &A, event: Event) -> IsUsed {
            let is_used = match event {
                Event::Command(cmd, _) => {
                    if data.is_empty() {
                        return Unused;
                    }
                    let (d_cols, d_rows) = data.len();
                    let (last_col, last_row) = (d_cols.wrapping_sub(1), d_rows.wrapping_sub(1));

                    let row_len: usize = self.cur_len.1.cast();
                    let mut solver = self.position_solver();
                    let (ci, ri) = match cx.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Unused,
                    };

                    use Command as C;
                    let data_index = match cmd {
                        C::DocHome => Some((0, 0)),
                        C::DocEnd => Some((last_col, last_row)),
                        C::Home => Some((0, ri)),
                        C::End => Some((last_col, ri)),
                        C::Left | C::WordLeft if ci > 0 => Some((ci - 1, ri)),
                        C::Up if ri > 0 => Some((ci, ri - 1)),
                        C::Right | C::WordRight if ci < last_col => Some((ci + 1, ri)),
                        C::Down if ri < last_row => Some((ci, ri + 1)),
                        C::PageUp if ri > 0 => Some((ci, ri.saturating_sub(row_len / 2))),
                        C::PageDown if ri < last_row => {
                            Some((ci, (ri + row_len / 2).min(last_row)))
                        }
                        // TODO: C::ViewUp, ...
                        _ => None,
                    };
                    return if let Some((ci, ri)) = data_index {
                        // Set nav focus and update scroll position
                        let action = self.scroll.focus_rect(cx, solver.rect(ci, ri), self.core.rect);
                        if !action.is_empty() {
                            cx.action(&self, action);
                            solver = self.update_widgets(&mut cx.config_cx(), data);
                        }

                        let index = solver.data_to_child(ci, ri);
                        #[cfg(debug_assertions)]
                        {
                            let rk = data
                                .row_iter_from(ri, 1)
                                .next()
                                .expect("data row len > data.row_iter_vec len");
                            let ck = data
                                .col_iter_from(ci, 1)
                                .next()
                                .expect("data col len > data.col_iter_vec len");
                            let key = data.make_key(&ck, &rk);
                            assert_eq!(
                                self.widgets[index].widget.id(),
                                key.make_id(self.id_ref()),
                            );
                        }

                        cx.next_nav_focus(self.widgets[index].widget.id(), false, FocusSource::Key);
                        Used
                    } else {
                        Unused
                    };
                }
                Event::PressStart { ref press } if press.is_primary() && cx.config().mouse_nav_focus() => {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|k| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                            cx.next_nav_focus(w.widget.id(), false, FocusSource::Pointer);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press.grab(self.id()).with_cx(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_gliding()
                            && w.key.as_ref().map(|k| k == key).unwrap_or(false)
                            && w.widget.rect().contains(press.coord + self.scroll.offset())
                        {
                            cx.push(kas::messages::Select);
                        }
                    }
                    Used
                }
                _ => Unused, // fall through to scroll handler
            };

            let (moved, used_by_sber) = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.core.rect);
            if moved {
                self.update_widgets(&mut cx.config_cx(), data);
            }
            is_used | used_by_sber
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &A) {
            let key;
            if let Some(index) = cx.last_child() {
                let w = &mut self.widgets[index];
                key = match w.key.clone() {
                    Some(k) => k,
                    None => return,
                };

                self.driver.on_messages(cx, &mut w.widget, data, &key);
            } else {
                // Message is from self
                key = match self.press_target.clone() {
                    Some((_, k)) => k,
                    None => return,
                };
            }

            if let Some(kas::messages::Select) = cx.try_pop() {
                match self.sel_mode {
                    SelectionMode::None => (),
                    SelectionMode::Single => {
                        cx.redraw(&self);
                        self.selection.clear();
                        self.selection.insert(key.clone());
                        cx.push(SelectionMsg::Select(key));
                    }
                    SelectionMode::Multiple => {
                        cx.redraw(&self);
                        if self.selection.remove(&key) {
                            cx.push(SelectionMsg::Deselect(key));
                        } else {
                            self.selection.insert(key.clone());
                            cx.push(SelectionMsg::Select(key));
                        }
                    }
                }
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &A, scroll: Scroll) {
            let act = self.scroll.scroll(cx, self.rect(), scroll);
            self.update_widgets(&mut cx.config_cx(), data);
            cx.action(self, act);
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = A;

        fn for_child_node(
            &mut self,
            data: &A,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                if let Some(ref key) = w.key {
                    if let Some(item) = data.borrow(key) {
                        closure(w.widget.as_node(item.borrow()));
                    }
                }
            }
        }

        fn _configure(&mut self, cx: &mut ConfigCx, data: &A, id: Id) {
            self.core.id = id;
            #[cfg(debug_assertions)]
            self.core.status.configure(&self.core.id);

            self.configure(cx);
            self.update(cx, data);
        }

        fn _update(&mut self, cx: &mut ConfigCx, data: &A) {
            #[cfg(debug_assertions)]
            self.core.status.update(&self.core.id);

            self.update(cx, data);
        }

        fn _send(
            &mut self,
            cx: &mut EventCx,
            data: &A,
            id: Id,
            event: Event,
        ) -> IsUsed {
            kas::impls::_send(self, cx, data, id, event)
        }

        fn _replay(&mut self, cx: &mut EventCx, data: &A, id: Id) {
            kas::impls::_replay(self, cx, data, id);
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            cx: &mut ConfigCx,
            data: &A,
            focus: Option<&Id>,
            advance: NavAdvance,
        ) -> Option<Id> {
            if cx.is_disabled(self.id_ref()) || self.cur_len == (0, 0) {
                return None;
            }

            let mut child = focus.and_then(|id| self.find_child_index(id));

            if let Some(index) = child {
                if let Some(Some(id)) = self.as_node(data)
                    .for_child(index, |mut w| w._nav_next(cx, focus, advance))
                {
                    return Some(id);
                }
            }

            let reverse = match advance {
                NavAdvance::None => return None,
                NavAdvance::Forward(_) => false,
                NavAdvance::Reverse(_) => true,
            };

            let mut starting_child = child;
            loop {
                let mut solver = self.position_solver();
                let (d_cols, d_rows) = data.len();
                let (ci, ri) = if let Some(index) = child {
                    let (ci, ri) = solver.child_to_data(index);
                    if !reverse {
                        if ci + 1 < d_cols {
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

                let action = self.scroll.self_focus_rect(solver.rect(ci, ri), self.core.rect);
                if !action.is_empty() {
                    cx.action(&self, action);
                    solver = self.update_widgets(cx, data);
                }

                let index = solver.data_to_child(ci, ri);
                if let Some(Some(id)) = self.as_node(data)
                    .for_child(index, |mut w| w._nav_next(cx, focus, advance))
                {
                    return Some(id);
                }

                child = Some(index);
                if starting_child == child {
                    return None;
                } else if starting_child.is_none() {
                    starting_child = child;
                }
            }
        }
    }
}

#[derive(Debug)]
struct PositionSolver {
    pos_start: Coord,
    skip: Size,
    size: Size,
    first_data: (u32, u32),
    cur_len: (u32, u32),
}

impl PositionSolver {
    /// Map a data index to child index
    fn data_to_child(&self, ci: usize, ri: usize) -> usize {
        let col_len: usize = self.cur_len.0.cast();
        let row_len: usize = self.cur_len.1.cast();
        (ci % col_len) + (ri % row_len) * col_len
    }

    /// Map a child index to `(col_index, row_index)`
    fn child_to_data(&self, index: usize) -> (usize, usize) {
        let col_len: usize = self.cur_len.0.cast();
        let row_len: usize = self.cur_len.1.cast();
        let first_col: usize = self.first_data.0.cast();
        let first_row: usize = self.first_data.1.cast();
        let col_start = (first_col / col_len) * col_len;
        let row_start = (first_row / row_len) * row_len;
        let mut col_index = col_start + index % col_len;
        let mut row_index = row_start + index / col_len;
        if col_index < first_col {
            col_index += col_len;
        }
        if row_index < first_row {
            row_index += row_len;
        }
        (col_index, row_index)
    }

    /// Rect of data item (ci, ri)
    fn rect(&self, ci: usize, ri: usize) -> Rect {
        let pos = self.pos_start + self.skip.cwise_mul(Size(ci.cast(), ri.cast()));
        Rect::new(pos, self.size)
    }
}
