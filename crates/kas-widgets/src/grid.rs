// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A grid widget

use kas::layout::{DynGridStorage, GridChildInfo, GridDimensions};
use kas::layout::{GridSetter, GridSolver, RulesSetter, RulesSolver};
use kas::{layout, prelude::*};
use std::ops::{Index, IndexMut};

/// A grid of boxed widgets
///
/// This is a parameterisation of [`Grid`]
/// This is parameterised over the handler message type.
///
/// See documentation of [`Grid`] type.
pub type BoxGrid<Data> = Grid<Box<dyn Widget<Data = Data>>>;

impl_scope! {
    /// A generic grid widget
    ///
    /// Child widgets are displayed in a grid, according to each child's
    /// [`GridChildInfo`]. This allows spans and overlapping widgets. The numbers
    /// of rows and columns is determined automatically while the sizes of rows and
    /// columns are determined based on their contents (including special handling
    /// for spans, *mostly* with good results).
    ///
    /// Note that all child widgets are stored in a list internally. The order of
    /// widgets in that list does not affect display position, but does have a few
    /// effects: (a) widgets may be accessed in this order via indexing, (b) widgets
    /// are configured and drawn in this order, (c) navigating
    /// through widgets with the Tab key currently uses the list order (though it
    /// may be changed in the future to use display order).
    ///
    /// There is no protection against multiple widgets occupying the same cell.
    /// If this does happen, the last widget in that cell will appear on top, but
    /// overlapping widget drawing may not be pretty.
    ///
    /// ## Alternatives
    ///
    /// Where the entries are fixed, also consider custom [`Widget`] implementations.
    ///
    /// ## Performance
    ///
    /// Most operations are `O(n)` in the number of children.
    #[autoimpl(Default)]
    #[widget]
    pub struct Grid<W: Widget> {
        core: widget_core!(),
        widgets: Vec<(GridChildInfo, W)>,
        data: DynGridStorage,
        dim: GridDimensions,
    }

    impl Widget for Self {
        type Data = W::Data;

        fn for_child_node(
            &mut self,
            data: &W::Data,
            index: usize,
            closure: Box<dyn FnOnce(Node<'_>) + '_>,
        ) {
            if let Some(w) = self.widgets.get_mut(index) {
                closure(w.1.as_node(data));
            }
        }
    }

    impl Layout for Self {
        #[inline]
        fn num_children(&self) -> usize {
            self.widgets.len()
        }
        fn get_child(&self, index: usize) -> Option<&dyn Layout> {
            self.widgets.get(index).map(|w| w.1.as_layout())
        }
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, &mut self.data);
            for (info, child) in &mut self.widgets {
                solver.for_child(&mut self.data, *info, |axis| {
                    child.size_rules(sizer.re(), axis)
                });
            }
            solver.finish(&mut self.data)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, &mut self.data);
            for (info, child) in &mut self.widgets {
                child.set_rect(cx, setter.child_rect(&mut self.data, *info));
            }
        }

        fn find_id(&mut self, coord: Coord) -> Option<Id> {
            if !self.rect().contains(coord) {
                return None;
            }
            self.widgets
                .iter_mut()
                .find_map(|(_, child)| child.find_id(coord))
                .or_else(|| Some(self.id()))
        }

        fn draw(&mut self, mut draw: DrawCx) {
            for (_, child) in &mut self.widgets {
                draw.recurse(child);
            }
        }
    }
}

impl<W: Widget> Grid<W> {
    /// Construct a new instance
    #[inline]
    pub fn new() -> Self {
        Self::new_vec(vec![])
    }

    /// Construct a new instance
    #[inline]
    pub fn new_vec(widgets: Vec<(GridChildInfo, W)>) -> Self {
        let mut grid = Grid {
            widgets,
            ..Default::default()
        };
        grid.calc_dim();
        grid
    }

    /// Get grid dimensions
    ///
    /// The numbers of rows, columns and spans is determined automatically.
    #[inline]
    pub fn dimensions(&self) -> GridDimensions {
        self.dim
    }

    /// Access layout storage
    ///
    /// Use [`Self::dimensions`] to get expected dimensions.
    #[inline]
    pub fn layout_storage(&mut self) -> &mut impl layout::GridStorage {
        &mut self.data
    }

    fn calc_dim(&mut self) {
        let mut dim = GridDimensions::default();
        for child in &self.widgets {
            dim.cols = dim.cols.max(child.0.col_end);
            dim.rows = dim.rows.max(child.0.row_end);
            if child.0.col_end - child.0.col > 1 {
                dim.col_spans += 1;
            }
            if child.0.row_end - child.0.row > 1 {
                dim.row_spans += 1;
            }
        }
        self.dim = dim;
    }

    /// Construct via a builder
    pub fn build<F: FnOnce(GridBuilder<W>)>(f: F) -> Self {
        let mut grid = Self::default();
        let _ = grid.edit(f);
        grid
    }

    /// Edit an existing grid via a builder
    ///
    /// This may be used to edit children before window construction. It may
    /// also be used from a running UI, but in this case a full reconfigure
    /// of the window's widgets is required (triggered by the the return
    /// value, [`Action::RECONFIGURE`]).
    pub fn edit<F: FnOnce(GridBuilder<W>)>(&mut self, f: F) -> Action {
        f(GridBuilder(&mut self.widgets));
        self.calc_dim();
        Action::RECONFIGURE
    }

    /// True if there are no child widgets
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Returns the number of child widgets
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Returns a reference to the child, if any
    pub fn get(&self, index: usize) -> Option<&W> {
        self.widgets.get(index).map(|t| &t.1)
    }

    /// Returns a mutable reference to the child, if any
    pub fn get_mut(&mut self, index: usize) -> Option<&mut W> {
        self.widgets.get_mut(index).map(|t| &mut t.1)
    }

    /// Iterate over childern
    pub fn iter(&self) -> impl Iterator<Item = &(GridChildInfo, W)> {
        ListIter {
            list: &self.widgets,
        }
    }

    /// Mutably iterate over childern
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (GridChildInfo, W)> {
        ListIterMut {
            list: &mut self.widgets,
        }
    }
}

pub struct GridBuilder<'a, W: Widget>(&'a mut Vec<(GridChildInfo, W)>);
impl<'a, W: Widget> GridBuilder<'a, W> {
    /// True if there are no child widgets
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of child widgets
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// into the list. See documentation of [`Vec::reserve`].
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Remove all child widgets
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Add a child widget
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    pub fn push(&mut self, info: GridChildInfo, widget: W) {
        self.0.push((info, widget));
    }

    /// Add a child widget to the given cell
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    pub fn push_cell(&mut self, col: u32, row: u32, widget: W) {
        let info = GridChildInfo::new(col, row);
        self.push(info, widget);
    }

    /// Add a child widget to the given cell, builder style
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    #[must_use]
    pub fn with_cell(self, col: u32, row: u32, widget: W) -> Self {
        self.with_cell_span(col, row, 1, 1, widget)
    }

    /// Add a child widget to the given cell, with spans
    ///
    /// Parameters `col_span` and `row_span` are the number of columns/rows
    /// spanned and should each be at least 1.
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    pub fn push_cell_span(&mut self, col: u32, row: u32, col_span: u32, row_span: u32, widget: W) {
        let info = GridChildInfo {
            col,
            col_end: col + col_span,
            row,
            row_end: row + row_span,
        };
        self.push(info, widget);
    }

    /// Add a child widget to the given cell, with spans, builder style
    ///
    /// Parameters `col_span` and `row_span` are the number of columns/rows
    /// spanned and should each be at least 1.
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    #[must_use]
    pub fn with_cell_span(
        mut self,
        col: u32,
        row: u32,
        col_span: u32,
        row_span: u32,
        widget: W,
    ) -> Self {
        self.push_cell_span(col, row, col_span, row_span, widget);
        self
    }

    /// Remove the last child widget
    ///
    /// Returns `None` if there are no children. Otherwise, this
    /// triggers a reconfigure before the next draw operation.
    pub fn pop(&mut self) -> Option<(GridChildInfo, W)> {
        self.0.pop()
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, info: GridChildInfo, widget: W) {
        self.0.insert(index, (info, widget));
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> (GridChildInfo, W) {
        self.0.remove(index)
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace(&mut self, index: usize, info: GridChildInfo, widget: W) -> (GridChildInfo, W) {
        let mut item = (info, widget);
        std::mem::swap(&mut item, &mut self.0[index]);
        item
    }

    /// Append child widgets from an iterator
    pub fn extend<T: IntoIterator<Item = (GridChildInfo, W)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }

    /// Resize, using the given closure to construct new widgets
    pub fn resize_with<F: Fn(usize) -> (GridChildInfo, W)>(&mut self, len: usize, f: F) {
        let l0 = self.0.len();
        if l0 > len {
            self.0.truncate(len);
        } else if l0 < len {
            self.0.reserve(len);
            for i in l0..len {
                self.0.push(f(i));
            }
        }
    }

    /// Retain only widgets satisfying predicate `f`
    ///
    /// See documentation of [`Vec::retain`].
    pub fn retain<F: FnMut(&(GridChildInfo, W)) -> bool>(&mut self, f: F) {
        self.0.retain(f);
    }

    /// Get the first index of a child occupying the given cell, if any
    pub fn find_child_cell(&self, col: u32, row: u32) -> Option<usize> {
        for (i, (info, _)) in self.0.iter().enumerate() {
            if info.col <= col && col < info.col_end && info.row <= row && row < info.row_end {
                return Some(i);
            }
        }
        None
    }

    /// Iterate over childern
    pub fn iter(&self) -> impl Iterator<Item = &(GridChildInfo, W)> {
        ListIter { list: self.0 }
    }

    /// Mutably iterate over childern
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (GridChildInfo, W)> {
        ListIterMut { list: self.0 }
    }
}

impl<W: Widget> FromIterator<(GridChildInfo, W)> for Grid<W> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (GridChildInfo, W)>,
    {
        Self::new_vec(iter.into_iter().collect())
    }
}

impl<W: Widget> Index<usize> for Grid<W> {
    type Output = (GridChildInfo, W);

    fn index(&self, index: usize) -> &Self::Output {
        &self.widgets[index]
    }
}

impl<W: Widget> IndexMut<usize> for Grid<W> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.widgets[index]
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [(GridChildInfo, W)],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a (GridChildInfo, W);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((first, rest)) = self.list.split_first() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIter<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}

struct ListIterMut<'a, W: Widget> {
    list: &'a mut [(GridChildInfo, W)],
}
impl<'a, W: Widget> Iterator for ListIterMut<'a, W> {
    type Item = &'a mut (GridChildInfo, W);
    fn next(&mut self) -> Option<Self::Item> {
        let list = std::mem::take(&mut self.list);
        if let Some((first, rest)) = list.split_first_mut() {
            self.list = rest;
            Some(first)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}
impl<'a, W: Widget> ExactSizeIterator for ListIterMut<'a, W> {
    fn len(&self) -> usize {
        self.list.len()
    }
}
