// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`Collection`] trait

use crate::layout::{GridCellInfo, GridDimensions};
use crate::{Node, Tile, Widget};
use kas_macros::impl_self;
use std::ops::RangeBounds;

/// A collection of (child) widgets
///
/// Essentially, a `Collection` is a list of widgets. Notable implementations are:
///
/// -   Slices `[W]` where `W: Widget`
/// -   Arrays `[W; N]` where `W: Widget` and `const N: usize`
/// -   [`Vec`]`<W>` where `W: Widget`
/// -   The output of [`kas::collection!`]. This macro constructs an anonymous
///     struct of widgets which implements `Collection`.
pub trait Collection {
    /// The associated data type
    type Data;

    /// True if the collection is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of widgets
    fn len(&self) -> usize;

    /// Get a widget as a [`Tile`]
    fn get_tile(&self, index: usize) -> Option<&dyn Tile>;

    /// Get a widget as a mutable [`Tile`]
    fn get_mut_tile(&mut self, index: usize) -> Option<&mut dyn Tile>;

    /// Operate on a widget as a [`Node`]
    fn child_node<'n>(&'n mut self, data: &'n Self::Data, index: usize) -> Option<Node<'n>>;

    /// Iterate over elements as [`Tile`] items within `range`
    ///
    /// Note: there is currently no mutable equivalent due to the streaming
    /// iterator problem.
    fn iter_tile(&self, range: impl RangeBounds<usize>) -> CollectionIterTile<'_, Self> {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        let start = match range.start_bound() {
            Included(start) => *start,
            Excluded(start) => *start + 1,
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(end) => *end + 1,
            Excluded(end) => *end,
            Unbounded => self.len(),
        };
        CollectionIterTile {
            start,
            end,
            collection: self,
        }
    }

    /// Binary searches this collection with a comparator function.
    ///
    /// Similar to [`slice::binary_search_by`][<[()]>::binary_search_by], the
    /// comparator function should return whether the element is `Less` than,
    /// `Equal` to, or `Greater` than the desired target, and the collection
    /// should be sorted by this comparator (if not, the result is meaningless).
    ///
    /// Returns:
    ///
    /// -   `Some(Ok(index))` if an `Equal` element is found at `index`
    /// -   `Some(Err(index))` if no `Equal` element is found; in this case such
    ///     an element could be inserted at `index`
    /// -   `None` if [`Collection::get_tile`] returns `None` for some
    ///     `index` less than [`Collection::len`]. This is an error case that
    ///     should not occur.
    fn binary_search_by<'a, F>(&'a self, mut f: F) -> Option<Result<usize, usize>>
    where
        F: FnMut(&'a dyn Tile) -> std::cmp::Ordering,
    {
        use std::cmp::Ordering::{Greater, Less};

        // INVARIANTS:
        // - 0 <= left <= left + size = right <= self.len()
        // - f returns Less for everything in self[..left]
        // - f returns Greater for everything in self[right..]
        let mut size = self.len();
        let mut left = 0;
        let mut right = size;
        while left < right {
            let mid = left + size / 2;

            let cmp = f(self.get_tile(mid)?);

            if cmp == Less {
                left = mid + 1;
            } else if cmp == Greater {
                right = mid;
            } else {
                return Some(Ok(mid));
            }

            size = right - left;
        }

        Some(Err(left))
    }
}

/// A collection with attached cell info
pub trait CellCollection: Collection {
    /// Get row/column info associated with cell at `index`
    fn cell_info(&self, index: usize) -> Option<GridCellInfo>;

    /// Iterate over [`GridCellInfo`] of elements within `range`
    fn iter_cell_info(&self, range: impl RangeBounds<usize>) -> CollectionIterCellInfo<'_, Self> {
        use std::ops::Bound::{Excluded, Included, Unbounded};
        let start = match range.start_bound() {
            Included(start) => *start,
            Excluded(start) => *start + 1,
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(end) => *end + 1,
            Excluded(end) => *end,
            Unbounded => self.len(),
        };
        CollectionIterCellInfo {
            start,
            end,
            collection: self,
        }
    }

    /// Get or calculate grid dimension info
    ///
    /// The default implementation calculates this from [`Self::cell_info`].
    fn grid_dimensions(&self) -> GridDimensions {
        let mut dim = GridDimensions::default();
        let (mut last_col, mut last_row) = (0, 0);
        for cell_info in self.iter_cell_info(..) {
            last_col = last_col.max(cell_info.last_col);
            last_row = last_row.max(cell_info.last_row);
            if cell_info.last_col > cell_info.col {
                dim.col_spans += 1;
            }
            if cell_info.last_row > cell_info.row {
                dim.row_spans += 1;
            }
        }
        dim.cols = last_col + 1;
        dim.rows = last_row + 1;
        dim
    }
}

#[impl_self]
mod CollectionIterTile {
    /// An iterator over a [`Collection`] as [`Tile`] elements
    pub struct CollectionIterTile<'a, C: Collection + ?Sized> {
        start: usize,
        end: usize,
        collection: &'a C,
    }

    impl Iterator for Self {
        type Item = &'a dyn Tile;

        fn next(&mut self) -> Option<Self::Item> {
            let index = self.start;
            if index < self.end {
                self.start += 1;
                self.collection.get_tile(index)
            } else {
                None
            }
        }
    }

    impl DoubleEndedIterator for Self {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.start < self.end {
                let index = self.end - 1;
                self.end = index;
                self.collection.get_tile(index)
            } else {
                None
            }
        }
    }

    impl ExactSizeIterator for Self {}
}

#[impl_self]
mod CollectionIterCellInfo {
    /// An iterator over a [`Collection`] as [`GridCellInfo`] elements
    pub struct CollectionIterCellInfo<'a, C: CellCollection + ?Sized> {
        start: usize,
        end: usize,
        collection: &'a C,
    }

    impl Iterator for Self {
        type Item = GridCellInfo;

        fn next(&mut self) -> Option<Self::Item> {
            let index = self.start;
            if index < self.end {
                self.start += 1;
                self.collection.cell_info(index)
            } else {
                None
            }
        }
    }

    impl DoubleEndedIterator for Self {
        fn next_back(&mut self) -> Option<Self::Item> {
            if self.start < self.end {
                let index = self.end - 1;
                self.end = index;
                self.collection.cell_info(index)
            } else {
                None
            }
        }
    }

    impl ExactSizeIterator for Self {}
}

macro_rules! impl_slice {
    (($($gg:tt)*) for $t:ty as $w:ident in $pat:pat) => {
        impl<$($gg)*> Collection for $t {
            type Data = W::Data;

            #[inline]
            fn len(&self) -> usize {
                <[_]>::len(self)
            }

            #[inline]
            fn get_tile(&self, index: usize) -> Option<&dyn Tile> {
                self.get(index).map(|$pat| $w as &dyn Tile)
            }

            #[inline]
            fn get_mut_tile(&mut self, index: usize) -> Option<&mut dyn Tile> {
                self.get_mut(index).map(|$pat| $w as &mut dyn Tile)
            }

            #[inline]
            fn child_node<'n>(
                &'n mut self,
                data: &'n Self::Data,
                index: usize,
            ) -> Option<Node<'n>> {
                self.get_mut(index).map(|$pat| $w.as_node(data))
            }

            #[inline]
            fn binary_search_by<'a, F>(&'a self, mut f: F) -> Option<Result<usize, usize>>
            where
                F: FnMut(&'a dyn Tile) -> std::cmp::Ordering,
            {
                Some(<[_]>::binary_search_by(self, move |$pat| f($w.as_tile())))
            }
        }
    };
}

// NOTE: If Rust had better lifetime analysis we could replace
// the following impls with a single one:
// impl<W: Widget, T: std::ops::Deref<Target = [W]> + ?Sized> Collection for T
impl_slice!((const N: usize, W: Widget) for [W; N] as w in w);
impl_slice!((W: Widget) for [W] as w in w);
impl_slice!((W: Widget) for Vec<W> as w in w);

impl_slice!((const N: usize, W: Widget) for [(GridCellInfo, W); N] as w in (_, w));
impl_slice!((W: Widget) for [(GridCellInfo, W)] as w in (_, w));
impl_slice!((W: Widget) for Vec<(GridCellInfo, W)> as w in (_, w));

impl<W: Widget, C: Collection> CellCollection for C
where
    C: std::ops::Deref<Target = [(GridCellInfo, W)]>,
{
    fn cell_info(&self, index: usize) -> Option<GridCellInfo> {
        self.get(index).map(|(info, _)| *info)
    }
}
