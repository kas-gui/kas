// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

#[allow(unused)] // doc links
use crate::event::Event;
use crate::event::EventMgr;
use crate::{autoimpl, WidgetId};
use std::borrow::{Borrow, BorrowMut};
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Bounds on the key type
pub trait DataKey: Clone + Debug + Default + PartialEq + Eq + 'static {
    /// Make a [`WidgetId`] for a key
    ///
    /// The result must be distinct from `parent`.
    /// Use [`WidgetId::make_child`].
    fn make_id(&self, parent: &WidgetId) -> WidgetId;

    /// Reconstruct a key from a [`WidgetId`]
    ///
    /// Where `child` is the output of [`Self::make_id`] for the same `parent`
    /// *or any [`WidgetId`] descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    ///
    /// See: [`WidgetId::next_key_after`], [`WidgetId::iter_keys_after`]
    fn reconstruct_key(parent: &WidgetId, child: &WidgetId) -> Option<Self>;
}

impl DataKey for () {
    fn make_id(&self, parent: &WidgetId) -> WidgetId {
        // We need a distinct child, so use index 0
        parent.make_child(0)
    }

    fn reconstruct_key(parent: &WidgetId, child: &WidgetId) -> Option<Self> {
        if child.next_key_after(parent) == Some(0) {
            Some(())
        } else {
            None
        }
    }
}

// NOTE: we cannot use this blanket impl without specialisation / negative impls
// impl<Key: Cast<usize> + Clone + Debug + PartialEq + Eq + 'static> DataKey for Key
impl DataKey for usize {
    fn make_id(&self, parent: &WidgetId) -> WidgetId {
        parent.make_child(*self)
    }

    fn reconstruct_key(parent: &WidgetId, child: &WidgetId) -> Option<Self> {
        child.next_key_after(parent)
    }
}

/// Trait for shared data
///
/// By design, all methods take only `&self` and only allow immutable access to
/// data. See also [`SharedDataMut`].
#[autoimpl(for<T: trait + ?Sized>
    &T, &mut T, std::rc::Rc<T>, std::sync::Arc<T>, Box<T>)]
pub trait SharedData: Debug {
    /// Key type
    type Key: DataKey;

    /// Item type
    type Item: Clone + Debug + 'static;

    /// A borrow of the item type
    ///
    /// This type must support [`Borrow`] over [`Self::Item`]. This is, for
    /// example, supported by `Self::Item` and `&Self::Item`.
    ///
    /// It is also recommended (but not required) that the type support
    /// [`std::ops::Deref`]: this allows easier usage of [`Self::borrow`].
    ///
    /// TODO(spec): once Rust supports some form of specialization, `AsRef` will
    /// presumably get blanket impls over `T` and `&T`, and will then be more
    /// appropriate to use than `Borrow`.
    type ItemRef<'b>: Borrow<Self::Item>
    where
        Self: 'b;

    /// Check whether a key has data
    fn contains_key(&self, key: &Self::Key) -> bool;

    /// Borrow an item by `key`
    ///
    /// Returns `None` if `key` has no associated item.
    ///
    /// Depending on the implementation, this may involve some form of lock
    /// such as `RefCell::borrow` or `Mutex::lock`. The implementation should
    /// panic on lock failure, not return `None`.
    fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>>;

    /// Access a borrow of an item
    ///
    /// This is a convenience method over [`Self::borrow`].
    fn with_ref<V>(&self, key: &Self::Key, f: impl FnOnce(&Self::Item) -> V) -> Option<V>
    where
        Self: Sized,
    {
        self.borrow(key).map(|borrow| f(borrow.borrow()))
    }

    /// Get data by key (clone)
    ///
    /// Returns `None` if `key` has no associated item.
    ///
    /// This has a default implementation over [`Self::borrow`].
    #[inline]
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.borrow(key).map(|r| r.borrow().to_owned())
    }
}

/// Trait for shared mutable data
///
/// By design, all methods take only `&self`: since data is shared, an internal
/// locking or synchronization mechanism is required (e.g. `RefCell` or `Mutex`).
#[autoimpl(for<T: trait + ?Sized>
    &T, &mut T, std::rc::Rc<T>, std::sync::Arc<T>, Box<T>)]
pub trait SharedDataMut: SharedData {
    /// A mutable borrow of the item type
    ///
    /// This type must support [`BorrowMut`] over [`SharedData::Item`]. This is, for
    /// example, supported by `&mut Self::Item`.
    ///
    /// It is also recommended (but not required) that the type support
    /// [`std::ops::DerefMut`]: this allows easier usage of [`Self::borrow_mut`].
    type ItemRefMut<'b>: BorrowMut<Self::Item>
    where
        Self: 'b;

    /// Mutably borrow an item by `key` and notify of an update
    ///
    /// Returns `None` if the data is by design not mutable or if `key` has no
    /// associated item. Otherwise, this notifies
    /// users of a data update (by calling [`EventMgr::update_all`] *and*
    /// incrementing the number returned by [`SharedData::version`]).
    ///
    /// Depending on the implementation, this may involve some form of lock
    /// such as `RefCell::borrow_mut` or `Mutex::lock`. The implementation
    /// should panic on lock failure, not return `None`.
    ///
    /// Note: implementations of the return type *might* rely on [`Drop`] for
    /// synchronization. Failing to drop the return value may thus cause errors.
    fn borrow_mut(&self, mgr: &mut EventMgr, key: &Self::Key) -> Option<Self::ItemRefMut<'_>>;

    /// Access a mutable borrow of an item
    ///
    /// This is a convenience method over [`Self::borrow_mut`].
    fn with_ref_mut<V>(
        &self,
        mgr: &mut EventMgr,
        key: &Self::Key,
        f: impl FnOnce(&mut Self::Item) -> V,
    ) -> Option<V>
    where
        Self: Sized,
    {
        self.borrow_mut(mgr, key)
            .map(|mut borrow| f(borrow.borrow_mut()))
    }

    /// Set an item
    ///
    /// This is a convenience method over [`Self::borrow_mut`].
    #[inline]
    fn set(&self, mgr: &mut EventMgr, key: &Self::Key, item: Self::Item) {
        if let Some(mut borrow) = self.borrow_mut(mgr, key) {
            *borrow.borrow_mut() = item;
        }
    }
}

/// Trait bound for viewable single data
///
/// This is automatically implemented for every type implementing `SharedData<()>`.
///
/// Provided implementations: [`SharedRc`](super::SharedRc),
/// [`SharedArc`](super::SharedArc).
// TODO(trait aliases): make this an actual trait alias
pub trait SingleData: SharedData<Key = ()> {}
impl<T: SharedData<Key = ()>> SingleData for T {}

/// Trait bound for mutable single data
///
/// This is automatically implemented for every type implementing `SharedDataMut<()>`.
///
/// Provided implementations: [`SharedRc`](super::SharedRc),
/// [`SharedArc`](super::SharedArc).
// TODO(trait aliases): make this an actual trait alias
pub trait SingleDataMut: SharedDataMut<Key = ()> {}
impl<T: SharedDataMut<Key = ()>> SingleDataMut for T {}

/// Trait for viewable data lists
///
/// Provided implementations: `[T]`, `Vec<T>`.
#[allow(clippy::len_without_is_empty)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, std::rc::Rc<T>, std::sync::Arc<T>, Box<T>)]
pub trait ListData: SharedData {
    type KeyIter<'b>: Iterator<Item = Self::Key>
    where
        Self: 'b;

    /// No data is available
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of data items available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> usize;

    /// Iterate over keys
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    #[inline]
    fn iter_limit(&self, limit: usize) -> Self::KeyIter<'_> {
        self.iter_from(0, limit)
    }

    /// Iterate over keys from an arbitrary start-point
    ///
    /// The result is the same as `self.iter_limit(start + limit).skip(start)`.
    fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_>;
}

/// Trait for viewable data matrices
///
/// Data matrices are a kind of table where each cell has the same type.
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, std::rc::Rc<T>, std::sync::Arc<T>, Box<T>)]
pub trait MatrixData: SharedData {
    /// Column key type
    type ColKey: DataKey;
    /// Row key type
    type RowKey: DataKey;

    type ColKeyIter<'b>: Iterator<Item = Self::ColKey>
    where
        Self: 'b;
    type RowKeyIter<'b>: Iterator<Item = Self::RowKey>
    where
        Self: 'b;

    /// No data is available
    fn is_empty(&self) -> bool;

    /// Number of `(cols, rows)` available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> (usize, usize);

    /// Iterate over column keys
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    #[inline]
    fn col_iter_limit(&self, limit: usize) -> Self::ColKeyIter<'_> {
        self.col_iter_from(0, limit)
    }

    /// Iterate over column keys from an arbitrary start-point
    ///
    /// The result is the same as `self.iter_limit(start + limit).skip(start)`.
    fn col_iter_from(&self, start: usize, limit: usize) -> Self::ColKeyIter<'_>;

    /// Iterate over row keys
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    #[inline]
    fn row_iter_limit(&self, limit: usize) -> Self::RowKeyIter<'_> {
        self.row_iter_from(0, limit)
    }

    /// Iterate over row keys from an arbitrary start-point
    ///
    /// The result is the same as `self.iter_limit(start + limit).skip(start)`.
    fn row_iter_from(&self, start: usize, limit: usize) -> Self::RowKeyIter<'_>;

    /// Make a key from parts
    fn make_key(col: &Self::ColKey, row: &Self::RowKey) -> Self::Key;
}
