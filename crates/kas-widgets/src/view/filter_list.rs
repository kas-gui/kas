// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list view widget

use kas::prelude::*;
use kas::updatable::filter::Filter;
use kas::updatable::{ListData, SingleData};
use std::cell::RefCell;
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

impl<T: ListData + 'static, F: Filter<T::Item> + SingleData> FilteredList<T, F> {
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
        for key in self.data.iter_vec(usize::MAX) {
            if let Some(item) = self.data.get_cloned(&key) {
                if self.filter.matches(item) {
                    view.1.push(key);
                }
            }
        }
    }
}

impl<T: ListData + 'static, F: Filter<T::Item> + SingleData> ListData for FilteredList<T, F> {
    type Key = T::Key;
    type Item = T::Item;

    fn update_on_handles(&self, mgr: &mut EventState, id: &WidgetId) {
        self.data.update_on_handles(mgr, id);
        self.filter.update_on_handles(mgr, id);
    }
    fn version(&self) -> u64 {
        let ver = self.data.version() + self.filter.version();
        if ver > self.view.borrow().0 {
            self.refresh(ver);
        }
        ver
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

    fn contains_key(&self, key: &Self::Key) -> bool {
        self.get_cloned(key).is_some()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        // Check the item against our filter (probably O(1)) instead of using
        // our filtered list (O(n) where n=self.len()).
        self.data
            .get_cloned(key)
            .filter(|item| self.filter.matches(item.clone()))
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        // Filtering does not affect result, but does affect the view
        if self
            .data
            .get_cloned(key)
            .map(|item| !self.filter.matches(item))
            .unwrap_or(true)
        {
            // Not previously visible: no update occurs
            return None;
        }

        self.data.update(key, value)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
        let end = self.len().min(start + limit);
        self.view.borrow().1[start..end].to_vec()
    }
}
