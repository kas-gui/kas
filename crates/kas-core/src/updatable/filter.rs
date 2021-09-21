// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter accessor

use super::ListData;
use crate::cast::Cast;
#[allow(unused)]
use crate::event::Manager;
use crate::event::UpdateHandle;
use crate::updatable::*;
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

#[derive(Clone, Debug)]
struct FilterView<T: ListData, F: Filter<T::Item>> {
    filter: F,
    view: Vec<T::Key>,
}

impl<T: ListData, F: Filter<T::Item>> FilterView<T, F> {
    fn refresh(&mut self, data: &T) {
        self.view.clear();
        for (key, item) in data.iter_vec(usize::MAX) {
            if self.filter.matches(item) {
                self.view.push(key);
            }
        }
    }
}

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
/// Note: only `Rc<FilteredList<T, F>>` implements [`ListData`]; the [`Rc`]
/// wrapper is required!
///
/// Warning: this implementation is `O(n)` where `n = data.len()` and not well
/// optimised, thus is expected to be slow on large data lists.
#[derive(Clone, Debug)]
pub struct FilteredList<T: ListData, F: Filter<T::Item>> {
    /// Direct access to unfiltered data
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    pub data: T,
    cell: RefCell<FilterView<T, F>>,
    update: UpdateHandle,
}

impl<T: ListData, F: Filter<T::Item>> FilteredList<T, F> {
    /// Construct and apply filter
    #[inline]
    pub fn new(data: T, filter: F) -> Self {
        let len = data.len().cast();
        let view = Vec::with_capacity(len);
        let cell = RefCell::new(FilterView { filter, view });
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
        self.cell.borrow_mut().refresh(&self.data);
        self.update
    }

    /// Update and apply the filter
    ///
    /// An update should be triggered using the returned handle.
    /// See [`FilteredList::refresh`].
    pub fn set_filter(&self, filter: F) -> UpdateHandle {
        let mut cell = self.cell.borrow_mut();
        cell.filter = filter;
        cell.refresh(&self.data);
        self.update
    }
}

impl<T: ListData, F: Filter<T::Item>> Updatable for FilteredList<T, F> {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }

    fn update_self(&self) -> Option<UpdateHandle> {
        self.cell.borrow_mut().refresh(&self.data);
        Some(self.update)
    }
}
impl<K, M, T: ListData + UpdatableHandler<K, M> + 'static, F: Filter<T::Item>>
    UpdatableHandler<K, M> for FilteredList<T, F>
{
    fn handle(&self, key: &K, msg: &M) -> Option<UpdateHandle> {
        self.data.handle(key, msg)
    }
}

impl<T: ListData + 'static, F: Filter<T::Item>> ListData for FilteredList<T, F> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        self.cell.borrow().view.len()
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        self.get_cloned(key).is_some()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        // Check the item against our filter (probably O(1)) instead of using
        // our filtered list (O(n) where n=self.len()).
        let cell = self.cell.borrow();
        self.data
            .get_cloned(key)
            .filter(|item| cell.filter.matches(item.clone()))
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        // Filtering does not affect result, but does affect the view
        let mut cell = self.cell.borrow_mut();
        if self
            .data
            .get_cloned(key)
            .map(|item| !cell.filter.matches(item))
            .unwrap_or(true)
        {
            // Not previously visible: no update occurs
            return None;
        }

        let new_visible = cell.filter.matches(value.clone());
        let result = self.data.update(key, value);
        if result.is_some() && !new_visible {
            // remove the updated item from our filtered list
            cell.view.retain(|item| item != key);
        }
        result
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        let cell = self.cell.borrow();
        let end = self.len().min(start + limit);
        if start >= end {
            return Vec::new();
        }
        let mut v = Vec::with_capacity(end - start);
        for k in &cell.view[start..end] {
            v.push((k.clone(), self.data.get_cloned(k).unwrap()));
        }
        v
    }
}
