// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`Collection`] trait

use crate::{Layout, Node, Widget};
use std::ops::RangeBounds;

/// A collection of (child) widgets
///
/// Essentially, implementating types are lists of widgets. Simple examples are
/// `Vec<W>` and `[W; N]` where `W: Widget` and `const N: usize`. A more complex
/// example would be a custom struct where each field is a widget.
pub trait Collection {
    /// The associated data type
    type Data;

    /// True if the collection is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The number of widgets
    fn len(&self) -> usize;

    /// Get a widget as a [`Layout`]
    fn get_layout(&self, index: usize) -> Option<&dyn Layout>;

    /// Get a widget as a mutable [`Layout`]
    fn get_mut_layout(&mut self, index: usize) -> Option<&mut dyn Layout>;

    /// Operate on a widget as a [`Node`]
    fn for_node(
        &mut self,
        data: &Self::Data,
        index: usize,
        closure: Box<dyn FnOnce(Node<'_>) + '_>,
    );

    /// Iterate over elements as [`Layout`] items within `range`
    ///
    /// Note: there is currently no mutable equivalent due to the streaming
    /// iterator problem.
    fn iter_layout(&self, range: impl RangeBounds<usize>) -> CollectionIterLayout<'_, Self> {
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
        CollectionIterLayout {
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
    /// -   `None` if [`Collection::get_layout`] returns `None` for some
    ///     `index` less than [`Collection::len`]. This is an error case that
    ///     should not occur.
    fn binary_search_by<'a, F>(&'a self, mut f: F) -> Option<Result<usize, usize>>
    where
        F: FnMut(&'a dyn Layout) -> std::cmp::Ordering,
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

            let cmp = f(self.get_layout(mid)?);

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

/// An iterator over a [`Collection`] as [`Layout`] elements
pub struct CollectionIterLayout<'a, C: Collection + ?Sized> {
    start: usize,
    end: usize,
    collection: &'a C,
}

impl<'a, C: Collection + ?Sized> Iterator for CollectionIterLayout<'a, C> {
    type Item = &'a dyn Layout;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.start;
        if index < self.end {
            self.start += 1;
            self.collection.get_layout(index)
        } else {
            None
        }
    }
}

impl<'a, C: Collection + ?Sized> DoubleEndedIterator for CollectionIterLayout<'a, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let index = self.end - 1;
            self.end = index;
            self.collection.get_layout(index)
        } else {
            None
        }
    }
}

impl<'a, C: Collection + ?Sized> ExactSizeIterator for CollectionIterLayout<'a, C> {}

macro_rules! impl_slice {
    (($($gg:tt)*) for $t:ty) => {
        impl<$($gg)*> Collection for $t {
            type Data = W::Data;

            #[inline]
            fn len(&self) -> usize {
                <[W]>::len(self)
            }

            #[inline]
            fn get_layout(&self, index: usize) -> Option<&dyn Layout> {
                self.get(index).map(|w| w as &dyn Layout)
            }

            #[inline]
            fn get_mut_layout(&mut self, index: usize) -> Option<&mut dyn Layout> {
                self.get_mut(index).map(|w| w as &mut dyn Layout)
            }

            #[inline]
            fn for_node(
                &mut self,
                data: &W::Data,
                index: usize,
                closure: Box<dyn FnOnce(Node<'_>) + '_>,
            ) {
                if let Some(w) = self.get_mut(index) {
                    closure(w.as_node(data));
                }
            }

            #[inline]
            fn binary_search_by<'a, F>(&'a self, mut f: F) -> Option<Result<usize, usize>>
            where
                F: FnMut(&'a dyn Layout) -> std::cmp::Ordering,
            {
                Some(<[W]>::binary_search_by(self, move |w| f(w.as_layout())))
            }
        }
    };
}

// NOTE: If Rust had better lifetime analysis we could replace
// the following impls with a single one:
// impl<W: Widget, T: std::ops::Deref<Target = [W]> + ?Sized> Collection for T
impl_slice!((const N: usize, W: Widget) for [W; N]);
impl_slice!((W: Widget) for [W]);
impl_slice!((W: Widget) for Vec<W>);
