// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter accessor

use super::Accessor;
use kas::conv::{Cast, Conv};
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
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
#[derive(Clone, Debug)]
pub struct FilterAccessor<I, T: Accessor<I>, F: Filter<T::Item>>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    /// Direct access to unfiltered data
    ///
    /// If adjusting this, one should call [`FilterAccessor::refresh`] after.
    pub data: T,
    filter: F,
    view: Vec<u32>,
    update: UpdateHandle,
    _i: std::marker::PhantomData<I>,
}

impl<I, T: Accessor<I>, F: Filter<T::Item>> FilterAccessor<I, T, F>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    /// Construct and apply filter
    #[inline]
    pub fn new(data: T, filter: F) -> Self {
        let view = Vec::with_capacity(data.len().cast());
        let mut s = FilterAccessor {
            data,
            filter,
            view,
            update: UpdateHandle::new(),
            _i: Default::default(),
        };
        let _ = s.refresh();
        s
    }

    /// Refresh the view
    ///
    /// Re-applies the filter (`O(n)` where `n` is the number of data elements).
    /// Calling this directly may be useful in case the data is modified.
    ///
    /// An update should be triggered using the returned handle.
    pub fn refresh(&mut self) -> UpdateHandle {
        self.view.clear();
        for i in 0..self.data.len().cast() {
            if self.filter.matches(self.data.get(i.cast())) {
                self.view.push(i);
            }
        }
        self.update
    }

    /// Update and apply the filter
    ///
    /// An update should be triggered using the returned handle.
    /// See [`FilterAccessor::refresh`].
    pub fn set_filter(&mut self, filter: F) -> UpdateHandle {
        self.filter = filter;
        self.refresh()
    }
}

impl<I, T: Accessor<I>, F: Filter<T::Item>> Accessor<I> for FilterAccessor<I, T, F>
where
    I: Cast<u32> + Cast<usize> + Conv<u32> + Conv<usize> + Debug + 'static,
{
    type Item = T::Item;
    fn len(&self) -> I {
        self.view.len().cast()
    }
    fn get(&self, index: I) -> Self::Item {
        self.data.get(self.view[Cast::<usize>::cast(index)].cast())
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }
}
