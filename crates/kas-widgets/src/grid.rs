// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A grid widget

use kas::layout::{DynGridStorage, GridCellInfo, GridDimensions};
use kas::layout::{GridSetter, GridSolver, RulesSetter, RulesSolver};
use kas::{CellCollection, layout, prelude::*};
use std::ops::{Index, IndexMut};

/// Make a [`Grid`] widget
///
/// Constructs a table with auto-determined number of rows and columns.
/// Cells may overlap, in which case behaviour is identical to [`float!`]: the
/// first declared item is on top.
///
/// # Syntax
///
/// > _Collection_ :\
/// > &nbsp;&nbsp; `collection!` `[` _ItemArms_<sup>\?</sup> `]`
/// >
/// > _ItemArms_ :\
/// > &nbsp;&nbsp; (_ItemArm_ `,`)<sup>\*</sup> _ItemArm_ `,`<sup>\?</sup>
/// >
/// > _ItemArm_ :\
/// > &nbsp;&nbsp; `(` _Column_ `,` _Row_ `)` `=>` _Item_
/// >
/// > _Column_, _Row_ :\
/// > &nbsp;&nbsp; _LitInt_ | ( _LitInt_ `..` `+` _LitInt_ ) | ( _LitInt_ `..`
/// > _LitInt_ ) | ( _LitInt_ `..=` _LitInt_ )
///
/// Here, _Column_ and _Row_ are selected via an index (from 0), a range of
/// indices, or a start + increment. For example, `2` = `2..+1` = `2..3` =
/// `2..=2` while `5..+2` = `5..7` = `5..=6`.
///
/// ## Stand-alone usage
///
/// When used as a stand-alone macro, `grid! [/* ... */]` is just syntactic
/// sugar for `Grid::new(kas::cell_collection! [/* ... */])`.
///
/// In this case, _Item_ may be:
///
/// -   A string literal (interpreted as a label widget), optionally followed by
///     an [`align`] or [`pack`] method call
/// -   An expression yielding an object implementing `Widget<Data = _A>`
///
/// In case all _Item_ instances are a string literal, the data type of the
/// `grid!` widget will be `()`; otherwise the data type of the widget is `_A`
/// where `_A` is a generic type parameter of the widget.
///
/// ## Usage within widget layout syntax
///
/// In this case, _Item_ uses [widget layout syntax]. This is broadly similar to
/// the above with a couple of exceptions:
///
/// -   Supported layout macros do not need to be imported to the module scope
/// -   An _Item_ may be a `#[widget]` field of the widget
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::grid! {
///     (0, 0) => "one",
///     (1, 0) => "two",
///     (0..2, 1) => "three",
/// };
/// ```
///
/// [widget layout syntax]: macro@kas::layout
/// [`align`]: crate::AdaptWidget::align
/// [`pack`]: crate::AdaptWidget::pack
/// [`float!`]: crate::float
#[macro_export]
macro_rules! grid {
    ( $( ($cc:expr, $rr:expr) => $ee:expr ),* ) => {
        $crate::Grid::new( ::kas::cell_collection! [ $( ($cc, $rr) => $ee ),* ] )
    };
    ( $( ($cc:expr, $rr:expr) => $ee:expr ),+ , ) => {
        $crate::Grid::new( ::kas::cell_collection! [ $( ($cc, $rr) => $ee ),+ ] )
    };
}

/// Define a [`Grid`] as a sequence of rows
///
/// This is just special convenience syntax for defining a [`Grid`]. See also
/// [`grid!`] documentation.
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::aligned_column! [
///     row!["one", "two"],
///     row!["three", "four"],
/// ];
/// ```
#[macro_export]
macro_rules! aligned_column {
    () => {
        $crate::Grid::new(::kas::cell_collection! [])
    };
    ($(row![$($ee:expr),* $(,)?]),+ $(,)?) => {
        $crate::Grid::new(::kas::cell_collection![aligned_column $(row![$($ee),*]),+])
    };
}

/// Define a [`Grid`] as a sequence of columns
///
/// This is just special convenience syntax for defining a [`Grid`]. See also
/// [`grid!`] documentation.
///
/// # Example
///
/// ```
/// let my_widget = kas_widgets::aligned_row! [
///     column!["one", "two"],
///     column!["three", "four"],
/// ];
/// ```
#[macro_export]
macro_rules! aligned_row {
    () => {
        $crate::Grid::new(::kas::cell_collection! [])
    };
    ($(column![$($ee:expr),* $(,)?]),+ $(,)?) => {
        $crate::Grid::new(::kas::cell_collection![aligned_row $(column![$($ee),*]),+])
    };
}

#[impl_self]
mod Grid {
    /// A generic grid widget
    ///
    /// Child widgets are displayed in a grid, according to each child's
    /// [`GridCellInfo`]. This allows spans and overlapping widgets. The numbers
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
    ///
    /// ## Example
    /// ```
    /// use kas::cell_collection;
    /// # use kas_widgets::Grid;
    /// let _grid = Grid::new(cell_collection! {
    ///     (0, 0) => "one",
    ///     (1, 0) => "two",
    ///     (0..2, 1) => "three",
    /// });
    /// ```
    #[widget]
    pub struct Grid<C: CellCollection> {
        core: widget_core!(),
        layout: DynGridStorage,
        dim: GridDimensions,
        #[collection]
        widgets: C,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let mut solver = GridSolver::<Vec<_>, Vec<_>, _>::new(axis, self.dim, &mut self.layout);
            for n in 0..self.widgets.len() {
                if let Some((info, child)) =
                    self.widgets.cell_info(n).zip(self.widgets.get_mut_tile(n))
                {
                    solver.for_child(&mut self.layout, info, |axis| {
                        child.size_rules(sizer.re(), axis)
                    });
                }
            }
            solver.finish(&mut self.layout)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, hints: AlignHints) {
            widget_set_rect!(rect);
            let mut setter = GridSetter::<Vec<_>, Vec<_>, _>::new(rect, self.dim, &mut self.layout);
            for n in 0..self.widgets.len() {
                if let Some((info, child)) =
                    self.widgets.cell_info(n).zip(self.widgets.get_mut_tile(n))
                {
                    child.set_rect(cx, setter.child_rect(&mut self.layout, info), hints);
                }
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            for n in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_tile(n) {
                    child.draw(draw.re());
                }
            }
        }
    }

    impl Tile for Self {
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::None
        }

        fn probe(&self, coord: Coord) -> Id {
            for n in 0..self.widgets.len() {
                if let Some(child) = self.widgets.get_tile(n) {
                    if let Some(id) = child.try_probe(coord) {
                        return id;
                    }
                }
            }
            self.id()
        }
    }
}

impl<C: CellCollection> Grid<C> {
    /// Construct a new instance
    #[inline]
    pub fn new(widgets: C) -> Self {
        Grid {
            core: Default::default(),
            layout: Default::default(),
            dim: widgets.grid_dimensions(),
            widgets,
        }
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
    pub fn layout_storage(&mut self) -> &mut (impl layout::GridStorage + use<C>) {
        &mut self.layout
    }
}

impl<W: Widget> Grid<Vec<(GridCellInfo, W)>> {
    /// Construct via a builder
    pub fn build<F: FnOnce(GridBuilder<W>)>(f: F) -> Self {
        let mut grid = Grid::new(vec![]);
        f(GridBuilder(&mut grid.widgets));
        grid.dim = grid.widgets.grid_dimensions();
        grid
    }

    /// Edit an existing grid via a builder
    ///
    /// This method will reconfigure `self` and all children.
    pub fn edit<F: FnOnce(GridBuilder<W>)>(&mut self, cx: &mut ConfigCx, data: &W::Data, f: F) {
        f(GridBuilder(&mut self.widgets));
        self.dim = self.widgets.grid_dimensions();
        let id = self.id();
        cx.configure(self.as_node(data), id);
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
    pub fn iter(&self) -> impl Iterator<Item = &(GridCellInfo, W)> {
        ListIter {
            list: &self.widgets,
        }
    }

    /// Mutably iterate over childern
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (GridCellInfo, W)> {
        ListIterMut {
            list: &mut self.widgets,
        }
    }
}

pub struct GridBuilder<'a, W: Widget>(&'a mut Vec<(GridCellInfo, W)>);
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
    pub fn push(&mut self, info: GridCellInfo, widget: W) {
        self.0.push((info, widget));
    }

    /// Add a child widget to the given cell
    ///
    /// The child is added to the end of the "list", thus appears last in
    /// navigation order.
    pub fn push_cell(&mut self, col: u32, row: u32, widget: W) {
        let info = GridCellInfo::new(col, row);
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
        let info = GridCellInfo {
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
    pub fn pop(&mut self) -> Option<(GridCellInfo, W)> {
        self.0.pop()
    }

    /// Inserts a child widget position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, info: GridCellInfo, widget: W) {
        self.0.insert(index, (info, widget));
    }

    /// Removes the child widget at position `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> (GridCellInfo, W) {
        self.0.remove(index)
    }

    /// Replace the child at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace(&mut self, index: usize, info: GridCellInfo, widget: W) -> (GridCellInfo, W) {
        let mut item = (info, widget);
        std::mem::swap(&mut item, &mut self.0[index]);
        item
    }

    /// Append child widgets from an iterator
    pub fn extend<T: IntoIterator<Item = (GridCellInfo, W)>>(&mut self, iter: T) {
        self.0.extend(iter);
    }

    /// Resize, using the given closure to construct new widgets
    pub fn resize_with<F: Fn(usize) -> (GridCellInfo, W)>(&mut self, len: usize, f: F) {
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
    pub fn retain<F: FnMut(&(GridCellInfo, W)) -> bool>(&mut self, f: F) {
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
    pub fn iter(&self) -> impl Iterator<Item = &(GridCellInfo, W)> {
        ListIter { list: self.0 }
    }

    /// Mutably iterate over childern
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (GridCellInfo, W)> + use<'_, W> {
        ListIterMut { list: self.0 }
    }
}

impl<W: Widget> FromIterator<(GridCellInfo, W)> for Grid<Vec<(GridCellInfo, W)>> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (GridCellInfo, W)>,
    {
        Self::new(iter.into_iter().collect())
    }
}

impl<W: Widget> Index<usize> for Grid<Vec<(GridCellInfo, W)>> {
    type Output = (GridCellInfo, W);

    fn index(&self, index: usize) -> &Self::Output {
        &self.widgets[index]
    }
}

impl<W: Widget> IndexMut<usize> for Grid<Vec<(GridCellInfo, W)>> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.widgets[index]
    }
}

struct ListIter<'a, W: Widget> {
    list: &'a [(GridCellInfo, W)],
}
impl<'a, W: Widget> Iterator for ListIter<'a, W> {
    type Item = &'a (GridCellInfo, W);
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
    list: &'a mut [(GridCellInfo, W)],
}
impl<'a, W: Widget> Iterator for ListIterMut<'a, W> {
    type Item = &'a mut (GridCellInfo, W);
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
