// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list adapter

use kas::model::filter::Filter;
use kas::model::{ListData, SharedData, SharedDataMut, SingleData};
use kas::prelude::*;
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
pub struct FilteredList<T: ListData, F: Filter<T::Item> + SingleData> {
    /// Direct access to unfiltered data
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    data: T,
    /// Direct access to the filter
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    filter: F,
    view: RefCell<(u64, Vec<T::Key>)>,
}

impl<T: ListData, F: Filter<T::Item> + SingleData> FilteredList<T, F> {
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
        for key in self.data.iter_limit(usize::MAX) {
            if let Some(item) = self.data.borrow(&key) {
                if self.filter.matches(item.borrow()) {
                    view.1.push(key);
                }
            }
        }
    }
}

impl<T: ListData, F: Filter<T::Item> + SingleData> SharedData for FilteredList<T, F> {
    type Key = T::Key;
    type Item = T::Item;
    type ItemRef<'b> = T::ItemRef<'b> where T: 'b;

    fn version(&self) -> u64 {
        let ver = self.data.version() + self.filter.version();
        if ver > self.view.borrow().0 {
            self.refresh(ver);
        }
        ver
    }

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

impl<T: ListData + SharedDataMut, F: Filter<T::Item> + SingleData> SharedDataMut
    for FilteredList<T, F>
{
    type ItemRefMut<'b> = T::ItemRefMut<'b> where T: 'b;

    fn borrow_mut(&self, mgr: &mut EventMgr, key: &Self::Key) -> Option<Self::ItemRefMut<'_>> {
        // Filtering does not affect result, but does affect the view
        if self
            .data
            .borrow(key)
            .map(|item| !self.filter.matches(item.borrow()))
            .unwrap_or(true)
        {
            // Not previously visible: no update occurs
            return None;
        }

        self.data.borrow_mut(mgr, key)
    }
}

impl<T: ListData, F: Filter<T::Item> + SingleData> ListData for FilteredList<T, F> {
    type KeyIter<'b> = KeyIter<'b, T::Key>
    where Self: 'b;

    fn is_empty(&self) -> bool {
        self.view.borrow().1.is_empty()
    }
    fn len(&self) -> usize {
        self.view.borrow().1.len()
    }
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        self.data.make_id(parent, key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        self.data.reconstruct_key(parent, child)
    }

    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
        let end = self.len().min(start + limit);
        let borrow = Ref::map(self.view.borrow(), |tuple| &tuple.1[start..end]);
        let index = 0;
        KeyIter { borrow, index }
    }
}

/// Key iterator used by [`FilteredList`]
pub struct KeyIter<'b, K: Clone> {
    borrow: Ref<'b, [K]>,
    index: usize,
}

impl<'b, K: Clone> Iterator for KeyIter<'b, K> {
    type Item = K;

    fn next(&mut self) -> Option<K> {
        let key = self.borrow.get(self.index).cloned();
        if key.is_some() {
            self.index += 1;
        }
        key
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.borrow().len() - self.index;
        (len, Some(len))
    }
}
impl<'b, K: Clone> ExactSizeIterator for KeyIter<'b, K> {}
impl<'b, K: Clone> std::iter::FusedIterator for KeyIter<'b, K> {}
