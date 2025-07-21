// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Grid view controller

use super::*;
use kas::NavAdvance;
use kas::event::components::ScrollComponent;
use kas::event::{Command, CursorIcon, FocusSource, Scroll, TimerHandle};
use kas::layout::{GridCellInfo, solve_size_rules};
use kas::prelude::*;
use kas::theme::SelectionStyle;
#[allow(unused)] // doc links
use kas_widgets::ScrollBars;
use linear_map::set::LinearSet;
use std::time::Instant;

const TIMER_UPDATE_WIDGETS: TimerHandle = TimerHandle::new(1, true);

#[derive(Clone, Copy, Debug, Default)]
struct Dim {
    rows: i32,
    cols: i32,
}

#[impl_self]
mod GridCell {
    /// A wrapper for selectable items
    ///
    /// This widget adds a thin frame around contents, supporting navigation
    /// focus and activation.
    ///
    /// # Messages
    ///
    /// When activated, this widget pushes [`Select`] to the message stack.
    ///
    /// [`Select`]: kas::messages::Select
    #[derive(Clone, Default)]
    #[widget]
    #[layout(frame!(self.inner).with_style(kas::theme::FrameStyle::NavFocus))]
    struct GridCell<W: Widget> {
        core: widget_core!(),
        col: u32,
        row: u32,
        selected: Option<bool>,
        /// The inner widget
        #[widget]
        inner: W,
    }

    impl Self {
        /// Construct a frame
        #[inline]
        fn new(inner: W) -> Self {
            GridCell {
                core: Default::default(),
                col: 0,
                row: 0,
                selected: None,
                inner,
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::GridCell {
                info: Some(GridCellInfo::new(self.col, self.row)),
                selected: self.selected,
            }
        }

        fn navigable(&self) -> bool {
            true
        }
    }

    impl Events for Self {
        type Data = W::Data;

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            match event {
                Event::Command(cmd, code) if cmd.is_activate() => {
                    cx.depress_with_key(self.id(), code);
                    cx.push(kas::messages::Select);
                    Used
                }
                _ => Unused,
            }
        }
    }
}

#[derive(Clone, Default)]
#[autoimpl(Debug ignore self.item where K: trait)]
struct WidgetData<K, W: Widget> {
    key: Option<K>,
    item: GridCell<W>,
}

/// Index of a grid cell
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GridIndex {
    pub col: u32,
    pub row: u32,
}

impl GridIndex {
    /// Zero
    pub const ZERO: GridIndex = GridIndex { col: 0, row: 0 };

    /// Construct, copying `x` to both fields
    pub const fn splat(x: u32) -> Self {
        GridIndex { col: x, row: x }
    }
}

impl crate::DataKey for GridIndex {
    fn make_id(&self, parent: &Id) -> Id {
        parent
            .make_child(self.col.cast())
            .make_child(self.row.cast())
    }

    fn reconstruct_key(parent: &Id, child: &Id) -> Option<Self> {
        let mut iter = child.iter_keys_after(parent);
        let col = iter.next().map(|i| i.cast())?;
        let row = iter.next().map(|i| i.cast())?;
        Some(GridIndex { col, row })
    }
}

#[impl_self]
mod GridView {
    /// View controller for 2D indexable data (grid)
    ///
    /// This widget generates a view over a list of data items via a
    /// [`DataClerk`]. "View widgets" are constructed via a [`Driver`]
    /// to represent visible data items. These view widgets are reassigned as
    /// required when the grid is scrolled, keeping the number of widgets in
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
    pub struct GridView<C: DataClerk<GridIndex>, V: Driver<C::Key, C::Item>> {
        core: widget_core!(),
        frame_offset: Offset,
        frame_size: Size,
        clerk: C,
        driver: V,
        widgets: Vec<WidgetData<C::Key, V::Widget>>,
        align_hints: AlignHints,
        ideal_len: Dim,
        alloc_len: Dim,
        data_len: Size,
        cur_len: GridIndex,
        first_data: GridIndex,
        child_size_min: Size,
        child_size_ideal: Size,
        child_inter_margin: Size,
        child_size: Size,
        scroll: ScrollComponent,
        sel_mode: SelectionMode,
        sel_style: SelectionStyle,
        // TODO(opt): replace selection list with RangeOrSet type?
        selection: LinearSet<C::Key>,
        press_target: Option<(usize, C::Key)>,
    }

    impl Self {
        /// Construct a new instance
        pub fn new(clerk: C, driver: V) -> Self {
            GridView {
                core: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                clerk,
                driver,
                widgets: Default::default(),
                align_hints: Default::default(),
                ideal_len: Dim { cols: 3, rows: 5 },
                alloc_len: Dim::default(),
                data_len: Size::ZERO,
                cur_len: GridIndex::ZERO,
                first_data: GridIndex::ZERO,
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

        /// Access the data clerk
        pub fn clerk(&self) -> &C {
            &self.clerk
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
        ///
        /// [`Select`]: kas::messages::Select
        pub fn set_selection_mode(&mut self, cx: &mut EventState, mode: SelectionMode) {
            self.sel_mode = mode;
            match mode {
                SelectionMode::None if !self.selection.is_empty() => {
                    self.selection.clear();
                    cx.redraw(self);
                }
                SelectionMode::Single if self.selection.len() > 1 => {
                    if let Some(first) = self.selection.iter().next().cloned() {
                        self.selection.retain(|item| *item == first);
                    }
                    cx.redraw(self);
                }
                _ => (),
            }
        }
        /// Set the initial selection mode (inline)
        ///
        /// See [`Self::set_selection_mode`] documentation.
        #[must_use]
        pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
            debug_assert!(self.selection.is_empty());
            self.sel_mode = mode;
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
        pub fn set_selection_style(&mut self, cx: &mut EventState, style: SelectionStyle) {
            if style.is_external() != self.sel_style.is_external() {
                cx.resize(&self);
            };
            self.sel_style = style;
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
        pub fn selected_iter(&'_ self) -> impl Iterator<Item = &'_ C::Key> + '_ {
            self.selection.iter()
        }

        /// Check whether an entry is selected
        pub fn is_selected(&self, key: &C::Key) -> bool {
            self.selection.contains(key)
        }

        /// Clear all selected items
        pub fn clear_selected(&mut self, cx: &mut EventState) {
            if !self.selection.is_empty() {
                self.selection.clear();
                cx.redraw(self);
            }
        }

        /// Directly select an item
        ///
        /// Does nothing if [`Self::selection_mode`] is [`SelectionMode::None`].
        /// Does not verify the validity of `key`.
        /// Does not send [`SelectionMsg`] messages.
        ///
        /// Returns `true` if newly selected, `false` if
        /// already selected. Fails if selection mode does not permit selection
        /// or if the key is invalid.
        pub fn select(&mut self, cx: &mut EventState, key: C::Key) -> bool {
            match self.sel_mode {
                SelectionMode::None => return false,
                SelectionMode::Single => self.selection.clear(),
                _ => (),
            }
            let r = self.selection.insert(key);
            if r {
                cx.redraw(self);
            }
            r
        }

        /// Directly deselect an item
        ///
        /// Returns `true` if deselected, `false` if not
        /// previously selected or if the key is invalid.
        pub fn deselect(&mut self, cx: &mut EventState, key: &C::Key) -> bool {
            let r = self.selection.remove(key);
            if r {
                cx.redraw(self);
            }
            r
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

        /// Widgets in the range `0..self.cur_end()` are currently in use
        #[inline]
        fn cur_end(&self) -> usize {
            usize::conv(self.cur_len.col) * usize::conv(self.cur_len.row)
        }

        fn position_solver(&self) -> PositionSolver {
            PositionSolver {
                pos_start: self.rect().pos + self.frame_offset,
                skip: self.child_size + self.child_inter_margin,
                size: self.child_size,
                first_data: self.first_data,
                cur_len: self.cur_len,
            }
        }

        // If full, call cx.update on all view widgets
        fn update_widgets(
            &mut self,
            cx: &mut ConfigCx,
            data: &C::Data,
            full: bool,
        ) -> PositionSolver {
            let time = Instant::now();

            let offset = self.scroll_offset();
            let skip = (self.child_size + self.child_inter_margin).max(Size(1, 1));
            let data_len = self.clerk.len(data);
            let col_len = data_len.col.min(self.alloc_len.cols.cast());
            let row_len = data_len.row.min(self.alloc_len.rows.cast());
            let first_col =
                u32::conv(u64::conv(offset.0) / u64::conv(skip.0)).min(data_len.col - col_len);
            let first_row =
                u32::conv(u64::conv(offset.1) / u64::conv(skip.1)).min(data_len.row - row_len);
            self.cur_len = GridIndex {
                col: col_len.cast(),
                row: row_len.cast(),
            };
            debug_assert!(self.cur_end() <= self.widgets.len());

            let start = GridIndex {
                col: first_col,
                row: first_row,
            };
            self.first_data = start;

            let end = GridIndex {
                col: first_col + col_len,
                row: first_row + row_len,
            };
            self.clerk.prepare_range(cx, self.id(), data, start..end);

            let solver = self.position_solver();
            for row in start.row..end.row {
                for col in start.col..end.col {
                    let cell = GridIndex { col, row };
                    let i = solver.data_to_child(cell);
                    if let Some(key) = self.clerk.key(data, cell) {
                        let id = key.make_id(self.id_ref());
                        let w = &mut self.widgets[i];
                        if w.key.as_ref() != Some(&key) {
                            self.driver.set_key(&mut w.item.inner, &key);

                            if let Some(item) = self.clerk.item(data, &key) {
                                cx.configure(w.item.as_node(item), id);

                                w.key = Some(key);
                                solve_size_rules(
                                    &mut w.item,
                                    cx.size_cx(),
                                    Some(self.child_size.0),
                                    Some(self.child_size.1),
                                );
                            } else {
                                w.key = None; // disables drawing and clicking
                            }
                        } else if full && let Some(item) = self.clerk.item(data, &key) {
                            cx.update(w.item.as_node(item));
                        }

                        if w.key.is_some() {
                            w.item.set_rect(cx, solver.rect(cell), self.align_hints);
                        }
                    } else {
                        self.widgets[i].key = None;
                    }
                }
            }

            let dur = (Instant::now() - time).as_micros();
            log::trace!(target: "kas_perf::view::grid_view", "update_widgets: {dur}μs");
            solver
        }

        fn update_content_size(&mut self, cx: &mut ConfigCx) {
            let view_size = self.rect().size - self.frame_size;
            let skip = self.child_size + self.child_inter_margin;
            let content_size =
                (skip.cwise_mul(self.data_len) - self.child_inter_margin).max(Size::ZERO);
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

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            if !action.is_empty() {
                cx.action(&self, action);
                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }
            self.scroll.offset()
        }
    }

    impl Layout for Self {
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
            axis = AxisInfo::new(axis.is_vertical(), other);

            let mut child_size_min = i32::MAX;
            let mut rules = SizeRules::EMPTY;
            for w in self.widgets.iter_mut() {
                if w.key.is_some() {
                    let child_rules = w.item.size_rules(sizer.re(), axis);
                    child_size_min = child_size_min.min(child_rules.min_size());
                    rules = rules.max(child_rules);
                }
            }
            if child_size_min == i32::MAX {
                child_size_min = 1;
            }
            child_size_min = child_size_min.max(1);
            self.child_size_min.set_component(axis, child_size_min);
            self.child_size_ideal
                .set_component(axis, rules.ideal_size().max(sizer.min_element_size()));

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
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            self.align_hints = hints;

            let avail = rect.size - self.frame_size;
            let child_size = Size(avail.0 / self.ideal_len.cols, avail.1 / self.ideal_len.rows)
                .clamp(self.child_size_min, self.child_size_ideal);
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
                    "set_rect: allocating widgets (old len = {avail_widgets}, new = {req_widgets})",
                );
                self.widgets.resize_with(req_widgets, || WidgetData {
                    key: None,
                    item: GridCell::new(self.driver.make(&C::Key::default())),
                });

                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }

            // Call set_rect on children. (This might sometimes be unnecessary,
            // except that the Layout::set_rect specification requires this
            // action and we cannot guarantee that the requested
            // TIMER_UPDATE_WIDGETS event will be immediately.)

            let col_len = self.cur_len.col;
            let row_len = self.cur_len.row;

            let pos_start = self.rect().pos + self.frame_offset;
            let skip = self.child_size + self.child_inter_margin;

            for rn in 0..row_len {
                let ri = self.first_data.row + rn;
                for cn in 0..col_len {
                    let ci = self.first_data.col + cn;
                    let i = usize::conv(ci % col_len)
                        + usize::conv(ri % row_len) * usize::conv(col_len);

                    let w = &mut self.widgets[i];
                    if w.key.is_some() {
                        let pos = pos_start + skip.cwise_mul(Size(ci.cast(), ri.cast()));
                        w.item
                            .set_rect(cx, Rect::new(pos, child_size), self.align_hints);
                    }
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let offset = self.scroll_offset();
            let rect = self.rect() + offset;
            let num = self.cur_end();
            draw.with_clip_region(self.rect(), offset, |mut draw| {
                for child in &self.widgets[..num] {
                    if let Some(ref key) = child.key {
                        // Note: we don't know which widgets within 0..num are
                        // visible, so check intersection before drawing:
                        if rect.intersection(&child.item.rect()).is_some() {
                            if self.selection.contains(key) {
                                draw.selection(child.item.rect(), self.sel_style);
                            }
                            child.item.draw(draw.re());
                        }
                    }
                }
            });
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::Grid {
                columns: Some(self.data_len.0.cast()),
                rows: Some(self.data_len.1.cast()),
            }
        }

        #[inline]
        fn child_indices(&self) -> ChildIndices {
            (0..self.cur_end()).into()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Tile> {
            self.widgets
                .get(index)
                .filter(|w| w.key.is_some())
                .map(|w| w.item.as_tile())
        }
        fn find_child_index(&self, id: &Id) -> Option<usize> {
            let key = C::Key::reconstruct_key(self.id_ref(), id);
            if key.is_some() {
                let num = self.cur_end();
                for (i, w) in self.widgets[..num].iter().enumerate() {
                    if key == w.key {
                        return Some(i);
                    }
                }
            }
            None
        }

        #[inline]
        fn translation(&self, _: usize) -> Offset {
            self.scroll_offset()
        }

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.is_kinetic_scrolling() {
                return self.id();
            }

            let num = self.cur_end();
            let coord = coord + self.scroll.offset();
            for child in &self.widgets[..num] {
                if child.key.is_some()
                    && let Some(id) = child.item.try_probe(coord)
                {
                    return id;
                }
            }
            self.id()
        }
    }

    impl Events for Self {
        fn hover_icon(&self) -> Option<CursorIcon> {
            self.scroll
                .is_kinetic_scrolling()
                .then_some(CursorIcon::AllScroll)
        }

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
                self.widgets.resize_with(len.cast(), || WidgetData {
                    key: None,
                    item: GridCell::new(self.driver.make(&C::Key::default())),
                });
                self.alloc_len = self.ideal_len;
            }

            cx.register_nav_fallback(self.id());
        }

        fn configure_recurse(&mut self, cx: &mut ConfigCx, data: &Self::Data) {
            let id = self.id();
            for w in &mut self.widgets {
                if let Some(ref key) = w.key
                    && let Some(item) = self.clerk.item(data, key)
                {
                    let id = key.make_id(&id);
                    cx.configure(w.item.as_node(item), id);
                }
            }
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &C::Data) {
            self.clerk.update(cx, self.id(), data);
            let len = self.clerk.len(data);
            let data_len = Size(len.col.cast(), len.row.cast());
            if data_len != self.data_len {
                self.data_len = data_len;
                // We must call at least SET_RECT to update scrollable region
                // RESIZE allows recalculation of child widget size which may
                // have been zero if no data was initially available!
                cx.resize(&self);
            }

            self.update_widgets(cx, data, true);
            self.update_content_size(cx);
        }

        fn update_recurse(&mut self, _: &mut ConfigCx, _: &Self::Data) {}

        fn handle_event(&mut self, cx: &mut EventCx, data: &C::Data, event: Event) -> IsUsed {
            let mut is_used = match event {
                Event::Command(cmd, _) => {
                    if self.data_len == Size::ZERO {
                        return Unused;
                    }
                    let len = self.clerk.len(data);
                    let (last_col, last_row) = (len.col.wrapping_sub(1), len.row.wrapping_sub(1));

                    let row_len = self.cur_len.row;
                    let mut solver = self.position_solver();
                    let cell = match cx.nav_focus().and_then(|id| self.find_child_index(id)) {
                        Some(index) => solver.child_to_data(index),
                        None => return Unused,
                    };
                    let (ci, ri) = (cell.col, cell.row);

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
                    return if let Some((col, row)) = data_index {
                        let cell = GridIndex { col, row };
                        // Set nav focus and update scroll position
                        let action = self.scroll.focus_rect(cx, solver.rect(cell), self.rect());
                        if !action.is_empty() {
                            cx.action(&self, action);
                            solver = self.update_widgets(&mut cx.config_cx(), data, false);
                        }

                        let index = solver.data_to_child(cell);
                        let w = &self.widgets[index];
                        #[cfg(debug_assertions)]
                        {
                            let key = self.clerk.key(data, cell).unwrap();
                            assert_eq!(w.key, Some(key));
                        }

                        if w.key.is_some() {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Key);
                        }
                        Used
                    } else {
                        Unused
                    };
                }
                Event::PressStart { ref press }
                    if press.is_primary() && cx.config().event().mouse_nav_focus() =>
                {
                    if let Some(index) = cx.last_child() {
                        self.press_target = self.widgets[index].key.clone().map(|k| (index, k));
                    }
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if w.key.as_ref().map(|k| k == key).unwrap_or(false) {
                            cx.next_nav_focus(w.item.id(), false, FocusSource::Pointer);
                        }
                    }

                    // Press may also be grabbed by scroll component (replacing
                    // this). Either way we can select on PressEnd.
                    press
                        .grab(self.id(), kas::event::GrabMode::Click)
                        .complete(cx)
                }
                Event::PressEnd { ref press, success } if press.is_primary() => {
                    if let Some((index, ref key)) = self.press_target {
                        let w = &mut self.widgets[index];
                        if success
                            && !matches!(self.sel_mode, SelectionMode::None)
                            && !self.scroll.is_kinetic_scrolling()
                            && w.key.as_ref().map(|k| k == key).unwrap_or(false)
                            && w.item.rect().contains(press.coord + self.scroll.offset())
                        {
                            cx.push(kas::messages::Select);
                        }
                    }
                    Used
                }
                Event::Timer(TIMER_UPDATE_WIDGETS) => {
                    self.update_widgets(&mut cx.config_cx(), data, false);
                    Used
                }
                _ => Unused, // fall through to scroll handler
            };

            let offset = self.scroll.offset();
            is_used |= self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.rect());
            if offset != self.scroll.offset() {
                // We may process multiple 'moved' events per frame; TIMER_UPDATE_WIDGETS will only
                // be processed once per frame.
                cx.request_frame_timer(self.id(), TIMER_UPDATE_WIDGETS);
            }
            is_used
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &C::Data) {
            let mut opt_key = None;
            if let Some(index) = cx.last_child() {
                // Message is from a child
                opt_key = match self.widgets[index].key.as_ref() {
                    Some(k) => Some(k.clone()),
                    None => return, // should be unreachable
                };
            }

            self.clerk
                .handle_messages(cx, self.id(), data, opt_key.as_ref());

            if let Some(kas::messages::Select) = cx.try_pop() {
                let key = match opt_key {
                    Some(key) => key,
                    None => match self.press_target.as_ref() {
                        Some((_, k)) => k.clone(),
                        None => return,
                    },
                };

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

        fn handle_scroll(&mut self, cx: &mut EventCx, data: &C::Data, scroll: Scroll) {
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);
            self.update_widgets(&mut cx.config_cx(), data, false);
        }
    }

    // Direct implementation of this trait outside of Kas code is not supported!
    impl Widget for Self {
        type Data = C::Data;

        fn child_node<'n>(&'n mut self, data: &'n C::Data, index: usize) -> Option<Node<'n>> {
            if let Some(w) = self.widgets.get_mut(index)
                && let Some(ref key) = w.key
                && let Some(item) = self.clerk.item(data, key)
            {
                return Some(w.item.as_node(item));
            }

            None
        }

        // Non-standard implementation to allow mapping new children
        fn _nav_next(
            &mut self,
            cx: &mut ConfigCx,
            data: &C::Data,
            focus: Option<&Id>,
            advance: NavAdvance,
        ) -> Option<Id> {
            if cx.is_disabled(self.id_ref()) || self.cur_len == GridIndex::ZERO {
                return None;
            }

            let mut child = focus.and_then(|id| self.find_child_index(id));

            if let Some(index) = child {
                let mut opt_id = None;
                let out = &mut opt_id;
                if let Some(mut node) = self.as_node(data).get_child(index) {
                    *out = node._nav_next(cx, focus, advance);
                }
                if let Some(id) = opt_id {
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
                let len = self.clerk.len(data);
                let mut cell;
                if let Some(index) = child {
                    cell = solver.child_to_data(index);
                    if !reverse {
                        if cell.col + 1 < len.col {
                            cell.col += 1;
                        } else if cell.row + 1 < len.row {
                            cell = GridIndex {
                                col: 0,
                                row: cell.row + 1,
                            };
                        } else {
                            return None;
                        }
                    } else {
                        if cell.col > 0 {
                            cell.col -= 1;
                        } else if cell.row > 0 {
                            cell = GridIndex {
                                col: len.col - 1,
                                row: cell.row - 1,
                            };
                        } else {
                            return None;
                        }
                    }
                } else if !reverse {
                    cell = GridIndex::ZERO;
                } else {
                    cell = GridIndex {
                        col: len.col - 1,
                        row: len.row - 1,
                    };
                }

                let action = self.scroll.self_focus_rect(solver.rect(cell), self.rect());
                if !action.is_empty() {
                    cx.action(&self, action);
                    solver = self.update_widgets(cx, data, false);
                }

                let index = solver.data_to_child(cell);

                let mut opt_id = None;
                let out = &mut opt_id;
                if let Some(mut node) = self.as_node(data).get_child(index) {
                    *out = node._nav_next(cx, focus, advance);
                }
                if let Some(id) = opt_id {
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
    first_data: GridIndex,
    cur_len: GridIndex,
}

impl PositionSolver {
    /// Map a data index to child index
    fn data_to_child(&self, cell: GridIndex) -> usize {
        let col_len: usize = self.cur_len.col.cast();
        let row_len: usize = self.cur_len.row.cast();
        (cell.col as usize % col_len) + (cell.row as usize % row_len) * col_len
    }

    /// Map a child index to a data index
    fn child_to_data(&self, index: usize) -> GridIndex {
        let col_len = self.cur_len.col;
        let row_len = self.cur_len.row;
        let ci: u32 = (index % usize::conv(col_len)).cast();
        let ri: u32 = (index / usize::conv(row_len)).cast();
        let mut col = (self.first_data.col / col_len) * col_len + ci;
        let mut row = (self.first_data.row / row_len) * row_len + ri;
        if col < self.first_data.col {
            col += col_len;
        }
        if row < self.first_data.row {
            row += row_len;
        }
        GridIndex { col, row }
    }

    /// Rect of data item (ci, ri)
    fn rect(&self, GridIndex { col, row }: GridIndex) -> Rect {
        let pos = self.pos_start + self.skip.cwise_mul(Size(col.cast(), row.cast()));
        Rect::new(pos, self.size)
    }
}
