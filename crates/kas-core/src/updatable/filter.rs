// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter accessor

use super::ListData;
use crate::cast::Cast;
#[allow(unused)]
use crate::event::Manager;
use crate::event::{UpdateHandle, VoidMsg};
use crate::updatable::*;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Types usable as a filter
pub trait Filter<T>: Updatable + 'static {
    /// Returns true if the given item matches this filter
    // TODO: once Accessor::get returns a reference, this should take item: &T where T: ?Sized
    fn matches(&self, item: T) -> bool;
}

/// Filter: target contains self (case-sensitive string match)
#[derive(Debug, Default, Clone)]
pub struct ContainsString(Rc<(UpdateHandle, RefCell<String>)>);

impl ContainsString {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let handle = UpdateHandle::new();
        let data = RefCell::new(s.to_string());
        ContainsString(Rc::new((handle, data)))
    }
}
impl Updatable for ContainsString {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some((self.0).0)
    }
}
impl UpdatableHandler<(), String> for ContainsString {
    fn handle(&self, _: &(), msg: &String) -> Option<UpdateHandle> {
        self.update(msg.clone())
    }
}
impl UpdatableHandler<(), VoidMsg> for ContainsString {
    fn handle(&self, _: &(), _: &VoidMsg) -> Option<UpdateHandle> {
        None
    }
}
impl SingleData for ContainsString {
    type Item = String;
    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().to_owned()
    }
    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        *(self.0).1.borrow_mut() = value;
        Some((self.0).0)
    }
}
impl SingleDataMut for ContainsString {
    fn set(&mut self, value: Self::Item) {
        *(self.0).1.borrow_mut() = value;
    }
}

impl<'a> Filter<&'a str> for ContainsString {
    fn matches(&self, item: &str) -> bool {
        item.contains(&self.get_cloned())
    }
}
impl Filter<String> for ContainsString {
    fn matches(&self, item: String) -> bool {
        Filter::<&str>::matches(self, &item)
    }
}

/// Filter: target contains self (case-insensitive string match)
///
// Note: the implemented method of caseless matching is not unicode compliant,
// however works in most cases (by converting both the source and the target to
// upper case). See [question on StackOverflow].
//
// [question on StackOverflow]: https://stackoverflow.com/questions/47298336/case-insensitive-string-matching-in-rust
#[derive(Debug, Default, Clone)]
pub struct ContainsCaseInsensitive(Rc<(UpdateHandle, RefCell<(String, String)>)>);

impl ContainsCaseInsensitive {
    /// Construct with given string
    pub fn new<S: ToString>(s: S) -> Self {
        let handle = UpdateHandle::new();
        let s = s.to_string();
        let u = s.to_uppercase();
        let data = RefCell::new((s, u));
        ContainsCaseInsensitive(Rc::new((handle, data)))
    }
}
impl Updatable for ContainsCaseInsensitive {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some((self.0).0)
    }
}
impl UpdatableHandler<(), String> for ContainsCaseInsensitive {
    fn handle(&self, _: &(), msg: &String) -> Option<UpdateHandle> {
        self.update(msg.clone())
    }
}
impl UpdatableHandler<(), VoidMsg> for ContainsCaseInsensitive {
    fn handle(&self, _: &(), _: &VoidMsg) -> Option<UpdateHandle> {
        None
    }
}
impl SingleData for ContainsCaseInsensitive {
    type Item = String;
    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().0.clone()
    }
    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 = cell.0.to_uppercase();
        Some((self.0).0)
    }
}
impl SingleDataMut for ContainsCaseInsensitive {
    fn set(&mut self, value: Self::Item) {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 = cell.0.to_uppercase();
    }
}

impl<'a> Filter<&'a str> for ContainsCaseInsensitive {
    fn matches(&self, item: &str) -> bool {
        Filter::<String>::matches(self, item.to_string())
    }
}
impl Filter<String> for ContainsCaseInsensitive {
    fn matches(&self, item: String) -> bool {
        item.to_uppercase().contains(&(self.0).1.borrow().1)
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
    /// Direct access to the filter
    ///
    /// If adjusting this, one should call [`FilteredList::refresh`] after.
    pub filter: F,
    view: RefCell<Vec<T::Key>>, // TODO: does this need to be in a RefCell?
}

impl<T: ListData, F: Filter<T::Item>> FilteredList<T, F> {
    /// Construct and apply filter
    #[inline]
    pub fn new(data: T, filter: F) -> Self {
        let len = data.len().cast();
        let view = RefCell::new(Vec::with_capacity(len));
        let s = FilteredList { data, filter, view };
        let _ = s.refresh();
        s
    }

    /// Refresh the view
    ///
    /// Re-applies the filter (`O(n)` where `n` is the number of data elements).
    /// Calling this directly may be useful in case the data is modified.
    ///
    /// An update should be triggered using the returned handle.
    pub fn refresh(&self) -> Option<UpdateHandle> {
        let mut view = self.view.borrow_mut();
        view.clear();
        for (key, item) in self.data.iter_vec(usize::MAX) {
            if self.filter.matches(item) {
                view.push(key);
            }
        }
        self.filter.update_handle()
    }
}

impl<T: ListData, F: Filter<T::Item>> Updatable for FilteredList<T, F> {
    fn update_handle(&self) -> Option<UpdateHandle> {
        self.filter.update_handle()
    }

    fn update_self(&self) -> Option<UpdateHandle> {
        self.refresh()
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
        self.view.borrow().len()
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

        let new_visible = self.filter.matches(value.clone());
        let result = self.data.update(key, value);
        if result.is_some() && !new_visible {
            // remove the updated item from our filtered list
            self.view.borrow_mut().retain(|item| item != key);
        }
        result
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        let view = self.view.borrow();
        let end = self.len().min(start + limit);
        if start >= end {
            return Vec::new();
        }
        let mut v = Vec::with_capacity(end - start);
        for k in &view[start..end] {
            v.push((k.clone(), self.data.get_cloned(k).unwrap()));
        }
        v
    }
}
