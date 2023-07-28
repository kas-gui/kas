// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Filter-list adapter

use super::Filter;
use crate::{ListData, SharedData};
use kas::event::{ConfigMgr, EventMgr};
use kas::{autoimpl, impl_scope, Widget};
use std::fmt::Debug;

#[derive(Debug, Default)]
pub struct SetFilter<T: Debug>(pub T);

impl_scope! {
    /// A widget adding a filter over some [`ListData`]
    ///
    /// Why is this a widget? Widgets can access and pass on data, which is
    /// what we need to filter a list.
    ///
    /// Warning: this implementation is at least `O(n)` where `n = data.len()`.
    /// Large collections may need to be filtered through another means.
    /// This design may be re-evaluated for performance in the future.
    ///
    /// To set the filter call [`Self::set_filter`] or pass a message of type
    /// `SetFilter<F::Value>`.
    #[autoimpl(Deref, DerefMut using self.inner)]
    #[autoimpl(Scrollable using self.inner where W: trait)]
    #[widget {
        layout = self.inner;
    }]
    pub struct FilterList<A: ListData + 'static, F: Filter<A::Item>, W: Widget<Data = UnsafeFilteredList<A>>> {
        core: widget_core!(),
        #[widget(unsafe { &UnsafeFilteredList::new(data, &self.view) })]
        pub inner: W,
        filter: F,
        view: Vec<A::Key>,
    }

    impl Self {
        /// Construct around `inner` widget with the given `filter`
        pub fn new(inner: W, filter: F) -> Self {
            FilterList {
                core: Default::default(),
                inner,
                filter,
                view: vec![],
            }
        }

        /// Set filter value
        pub fn set_filter(&mut self, data: &A, mgr: &mut ConfigMgr, filter: F::Value) {
            if self.filter.set_filter(filter) {
                mgr.update(self.as_node_mut(data));
            }
        }
    }

    impl kas::Events for Self {
        type Data = A;

        fn update(&mut self, data: &A, _: &mut kas::event::ConfigMgr) {
            self.view.clear();
            self.view.reserve(data.len());
            for key in data.iter_from(0, usize::MAX) {
                if let Some(item) = data.borrow(&key) {
                    if self.filter.matches(std::borrow::Borrow::borrow(&item)) {
                        self.view.push(key);
                    }
                }
            }
        }

        fn handle_messages(&mut self, data: &A, mgr: &mut EventMgr) {
            if let Some(SetFilter(value)) = mgr.try_pop() {
                mgr.config_mgr(|mgr| self.set_filter(data, mgr, value));
            }
        }
    }
}

impl_scope! {
    /// Filtered view over a list
    ///
    /// WARNING: this struct is `unsafe` because it contains lifetime-bound
    /// references cast to `'static`. Instances or copies of this struct must
    /// not outlive functions they are passed into.
    /// (This is a poor design since it does not properly capsulate unsafety,
    /// used for compatibility with other components. It does at least
    /// encapsulate unsafety since this struct is only accessible behind a
    /// non-`mut` reference, cannot be copied, and none of its methods return
    /// references which don't have their own lifetime bound. Eventually the
    /// plan is to make `Widget::Data` a GAT (once Rust supports object-safe
    /// GAT traits), after which this struct may have a lifetime bound.)
    ///
    /// This is an abstraction over a [`ListData`]. Items and associated keys
    /// are not adjusted in any way.
    ///
    /// The filter applies to [`SharedData::contains_key`] and [`ListData`]
    /// methods, but not to [`SharedData::borrow`] (the latter can thus access
    /// items excluded by the filter).
    #[derive(Debug)]
    pub struct UnsafeFilteredList<A: ListData + 'static> {
        data: &'static A,
        view: &'static [A::Key],
    }

    impl Self {
        unsafe fn new<'a>(data: &'a A, view: &'a [A::Key]) -> Self {
            UnsafeFilteredList {
                data: std::mem::transmute(data),
                view: std::mem::transmute(view),
            }
        }
    }

    impl SharedData for Self {
        type Key = A::Key;
        type Item = A::Item;
        type ItemRef<'b> = A::ItemRef<'b> where A: 'b;

        fn contains_key(&self, key: &Self::Key) -> bool {
            // TODO(opt): note that this is O(n*n). For large lists it would be
            // faster to re-evaluate the filter. Alternatively we could use a
            // HashSet or BTreeSet to test membership.
            self.view.iter().any(|item| *item == *key)
        }
        #[inline]
        fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
            self.data.borrow(key)
        }
    }

    impl ListData for Self {
        type KeyIter<'b> = KeyIter<'b, A::Key>
        where Self: 'b;

        fn is_empty(&self) -> bool {
            self.view.is_empty()
        }
        fn len(&self) -> usize {
            self.view.len()
        }

        fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
            let end = self.len().min(start + limit);
            KeyIter { list: &self.view[start..end], index: 0 }
        }
    }
}

/// Key iterator used by [`FilteredList`]
pub struct KeyIter<'b, Item: Clone> {
    list: &'b [Item],
    index: usize,
}

impl<'b, Item: Clone> Iterator for KeyIter<'b, Item> {
    type Item = Item;

    fn next(&mut self) -> Option<Item> {
        let key = self.list.get(self.index).cloned();
        if key.is_some() {
            self.index += 1;
        }
        key
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.list.len() - self.index;
        (len, Some(len))
    }
}
impl<'b, Item: Clone> ExactSizeIterator for KeyIter<'b, Item> {}
impl<'b, Item: Clone> std::iter::FusedIterator for KeyIter<'b, Item> {}
