// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list adapter

use super::Filter;
use crate::{ListData, SharedData};
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::fmt::Debug;

/// Filter accessor over another accessor
///
/// This is an abstraction over a [`ListData`], applying a filter to items when
/// iterating and accessing.
///
/// When updating, the filter applies to the old value: if the old is included,
/// it is replaced by the new, otherwise no replacement occurs.
///
/// Note: the key and item types are the same as those in the underlying list,
/// thus one can also retrieve values from the underlying list directly.
///
/// Warning: this implementation is `O(n)` where `n = data.len()` and not well
/// optimised, thus is expected to be slow on large data lists.
#[derive(Clone, Debug)]
pub struct FilteredList<T: ListData, F: Filter<T::Item> + Debug> {
    /// Direct access to unfiltered data
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    data: T,
    /// Direct access to the filter
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    filter: F,
    view: RefCell<(u64, Vec<(T::Key, T::Version)>)>,
}

impl<T: ListData, F: Filter<T::Item> + Debug> FilteredList<T, F> {
    /// Construct from `data` and a `filter`
    #[inline]
    pub fn new(data: T, filter: F) -> Self {
        let len = data.len();
        let view = RefCell::new((0, Vec::with_capacity(len)));
        FilteredList { data, filter, view }
    }

    /// Refresh the view
    ///
    /// Re-applies the filter (`O(n)` where `n` is the number of data elements).
    /// Calling this directly may be useful in case the data is modified.
    fn refresh(&self, ver: u64) {
        let mut view = self.view.borrow_mut();
        view.0 = ver;
        view.1.clear();
        for (key, version) in self.data.iter_from(0, usize::MAX) {
            if let Some(item) = self.data.borrow(&key) {
                if self.filter.matches(item.borrow()) {
                    view.1.push((key, version));
                }
            }
        }
    }
}

impl<T: ListData, F: Filter<T::Item> + Debug> SharedData for FilteredList<T, F> {
    type Key = T::Key;
    type Version = T::Version;
    type Item = T::Item;
    type ItemRef<'b> = T::ItemRef<'b> where T: 'b;

    fn contains_key(&self, key: &Self::Key) -> bool {
        SharedData::borrow(self, key).is_some()
    }
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
        // Check the item against our filter (probably O(1)) instead of using
        // our filtered list (O(n) where n=self.len()).
        self.data
            .borrow(key)
            .filter(|item| self.filter.matches(item.borrow()))
    }
}

impl<T: ListData, F: Filter<T::Item> + Debug> ListData for FilteredList<T, F> {
    type KeyIter<'b> = KeyIter<'b, (T::Key, T::Version)>
    where Self: 'b;

    fn is_empty(&self) -> bool {
        self.view.borrow().1.is_empty()
    }
    fn len(&self) -> usize {
        self.view.borrow().1.len()
    }

    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
        let end = self.len().min(start + limit);
        let borrow = Ref::map(self.view.borrow(), |tuple| &tuple.1[start..end]);
        let index = 0;
        KeyIter { borrow, index }
    }
}

/// Key iterator used by [`FilteredList`]
pub struct KeyIter<'b, Item: Clone> {
    borrow: Ref<'b, [Item]>,
    index: usize,
}

impl<'b, Item: Clone> Iterator for KeyIter<'b, Item> {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        let key = self.borrow.get(self.index).cloned();
        if key.is_some() {
            self.index += 1;
        }
        key
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.borrow.len() - self.index;
        (len, Some(len))
    }
}
impl<'b, Item: Clone> ExactSizeIterator for KeyIter<'b, Item> {}
impl<'b, Item: Clone> std::iter::FusedIterator for KeyIter<'b, Item> {}
