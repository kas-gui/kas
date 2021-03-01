// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter accessor

use super::ListData;
use kas::conv::Cast;
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;

/// Types usable as a filter
pub trait Filter<T>: Debug + 'static {
    /// Returns true if the given item matches this filter
    // TODO: once Accessor::get returns a reference, this should take item: &T where T: ?Sized
    fn matches(&self, item: T) -> bool;
}
impl<'a, T: Clone, X> Filter<&'a T> for X
where
    X: Filter<T>,
{
    fn matches(&self, item: &T) -> bool {
        self.matches(item.clone())
    }
}

impl<'a> Filter<&'a str> for &'static str {
    fn matches(&self, item: &str) -> bool {
        item.contains(self)
    }
}
impl<'a> Filter<&'a str> for String {
    fn matches(&self, item: &str) -> bool {
        item.contains(self)
    }
}
impl Filter<String> for String {
    fn matches(&self, item: String) -> bool {
        item.contains(self)
    }
}

/// Case-insensitive string matcher
///
/// This type will likely be removed at some point since it is inefficient and
/// not accurate for all Unicode input.
#[derive(Clone, Debug)]
pub struct SimpleCaseInsensitiveFilter(String);
impl SimpleCaseInsensitiveFilter {
    /// Construct
    pub fn new<T: ToString>(filter: T) -> Self {
        // Note: this method of caseless matching is not unicode compliant!
        // https://stackoverflow.com/questions/47298336/case-insensitive-string-matching-in-rust
        SimpleCaseInsensitiveFilter(filter.to_string().to_uppercase())
    }
}
impl<'a> Filter<&'a str> for SimpleCaseInsensitiveFilter {
    fn matches(&self, item: &str) -> bool {
        item.to_owned().to_uppercase().contains(&self.0)
    }
}
impl Filter<String> for SimpleCaseInsensitiveFilter {
    fn matches(&self, item: String) -> bool {
        item.to_uppercase().contains(&self.0)
    }
}

/// Filter accessor over another accessor
///
/// Warning: the underlying data may have a separate update handle, and handles
/// are not currently transitive. That is, `FilterList`'s update handle is not
/// triggered by changes to the underlying data list.
///
/// Warning: this implementation is `O(n)` where `n = data.len()` and not well
/// optimised, thus is expected to be slow on large data lists.
///
/// Note: the key and item types are the same as those in the underlying list,
/// thus one can retrieve values from the underlying list directly (without
/// filtering).
#[derive(Clone, Debug)]
pub struct FilteredList<T: ListData, F: Filter<T::Item>> {
    /// Direct access to unfiltered data
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    pub data: T,
    cell: RefCell<(F, Vec<T::Key>)>,
    update: UpdateHandle,
}

impl<T: ListData, F: Filter<T::Item>> FilteredList<T, F> {
    /// Construct and apply filter
    #[inline]
    pub fn new(data: T, filter: F) -> Self {
        let len = data.len().cast();
        let view = Vec::with_capacity(len);
        let cell = RefCell::new((filter, view));
        // TODO: using a separate update handle allows notification of the
        // filter view update without notifying users of the underlying list,
        // *but* update of the list should also imply update of the view.
        // Can we make one update handle imply another?
        let update = UpdateHandle::new();
        let s = FilteredList { data, cell, update };
        let _ = s.refresh();
        s
    }

    /// Refresh the view
    ///
    /// Re-applies the filter (`O(n)` where `n` is the number of data elements).
    /// Calling this directly may be useful in case the data is modified.
    ///
    /// An update should be triggered using the returned handle.
    pub fn refresh(&self) -> UpdateHandle {
        let mut cell = self.cell.borrow_mut();
        Self::rebuild_view(&self.data, &mut cell);
        self.update
    }

    /// Update and apply the filter
    ///
    /// An update should be triggered using the returned handle.
    /// See [`FilteredList::refresh`].
    pub fn set_filter(&self, filter: F) -> UpdateHandle {
        let mut cell = self.cell.borrow_mut();
        cell.0 = filter;
        Self::rebuild_view(&self.data, &mut cell);
        self.update
    }

    fn rebuild_view(data: &T, cell: &mut (F, Vec<T::Key>)) {
        cell.1.clear();
        for (key, item) in data.iter_vec(usize::MAX) {
            if cell.0.matches(item) {
                cell.1.push(key);
            }
        }
    }
}

impl<T: ListData, F: Filter<T::Item>> ListData for FilteredList<T, F> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        self.cell.borrow().1.len()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        // We check the filter against the data (O(1)) instead of checking the
        // key against our view (O(len(view))).
        let cell = self.cell.borrow();
        self.data
            .get_cloned(key)
            .filter(|item| cell.0.matches(item.clone()))
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        let cell = self.cell.borrow();
        let end = self.len().min(start + limit);
        if start >= end {
            return Vec::new();
        }
        let mut v = Vec::with_capacity(end - start);
        for k in &cell.1[start..end] {
            v.push((k.clone(), self.data.get_cloned(k).unwrap()));
        }
        v
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }
}
