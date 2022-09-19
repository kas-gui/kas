// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

#[allow(unused)] // doc links
use crate::event::Event;
use crate::event::EventMgr;
use crate::macros::autoimpl;
use crate::WidgetId;
use std::borrow::Borrow;
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Bounds on the key type
pub trait DataKey: Clone + Debug + PartialEq + Eq + 'static {}
impl<Key: Clone + Debug + PartialEq + Eq + 'static> DataKey for Key {}

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

    /// Get the data version
    ///
    /// The version is increased on change and may be used to detect when views
    /// over the data need to be refreshed. The initial version number must be
    /// at least 1 (allowing 0 to represent an uninitialized state).
    ///
    /// Whenever the data is updated, [`Event::Update`] must be sent via
    /// [`EventMgr::update_all`] to notify other users of this data of the
    /// update.
    fn version(&self) -> u64;

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
    /// Update data, if supported
    ///
    /// Shared data with internal mutability (e.g. via [`RefCell`]) should
    /// update itself here, increase its version number and call
    /// [`EventMgr::update_all`].
    ///
    /// Data types without internal mutability should do nothing.
    fn update(&self, mgr: &mut EventMgr, key: &Self::Key, item: Self::Item);
}

/// Trait bound for viewable single data
///
/// This is automatically implemented for every type implementing `SharedData<()>`.
// TODO(trait aliases): make this an actual trait alias
pub trait SingleData: SharedData<Key = ()> {}
impl<T: SharedData<Key = ()>> SingleData for T {}

/// Trait bound for mutable single data
///
/// This is automatically implemented for every type implementing `SharedDataMut<()>`.
// TODO(trait aliases): make this an actual trait alias
pub trait SingleDataMut: SharedDataMut<Key = ()> {}
impl<T: SharedDataMut<Key = ()>> SingleDataMut for T {}

/// Trait for viewable data lists
#[allow(clippy::len_without_is_empty)]
#[autoimpl(for<T: trait + ?Sized> &T, &mut T, std::rc::Rc<T>, std::sync::Arc<T>, Box<T>)]
pub trait ListData: SharedData {
    /// No data is available
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of data items available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> usize;

    /// Make a [`WidgetId`] for a key
    ///
    /// Suggested impl, converting `key` as necessary:
    ///
    /// -   `parent.make_child(key)`
    ///
    /// See: [`WidgetId::make_child`]
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId;

    /// Reconstruct a key from a [`WidgetId`]
    ///
    /// Where `child` is the output of [`Self::make_id`] for the same `parent`
    /// *or any [`WidgetId`] descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    ///
    /// See: [`WidgetId::next_key_after`], [`WidgetId::iter_keys_after`]
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key>;

    // TODO(gat): replace with an iterator
    /// Iterate over keys as a vec
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
        self.iter_vec_from(0, limit)
    }

    /// Iterate over keys as a vec
    ///
    /// The result is the same as `self.iter_vec(start + limit).skip(start)`.
    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key>;
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

    /// No data is available
    fn is_empty(&self) -> bool;

    /// Number of `(cols, rows)` available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> (usize, usize);

    /// Make a [`WidgetId`] for a key
    ///
    /// Suggested impls, converting keys as necessary:
    ///
    /// -   `parent.make_child(combined_key)`
    /// -   `parent.make_child(col_key).make_child(row_key)`
    ///
    /// See: [`WidgetId::make_child`]
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId;

    /// Reconstruct a key from a [`WidgetId`]
    ///
    /// Where `child` is the output of [`Self::make_id`] for the same `parent`
    /// *or any [`WidgetId`] descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    ///
    /// See: [`WidgetId::next_key_after`], [`WidgetId::iter_keys_after`]
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key>;

    // TODO(gat): replace with an iterator
    /// Iterate over column keys as a vec
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    fn col_iter_vec(&self, limit: usize) -> Vec<Self::ColKey> {
        self.col_iter_vec_from(0, limit)
    }
    /// Iterate over column keys as a vec
    ///
    /// The result is the same as `self.iter_vec(start + limit).skip(start)`.
    fn col_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::ColKey>;

    /// Iterate over row keys as a vec
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    fn row_iter_vec(&self, limit: usize) -> Vec<Self::RowKey> {
        self.row_iter_vec_from(0, limit)
    }
    /// Iterate over row keys as a vec
    ///
    /// The result is the same as `self.iter_vec(start + limit).skip(start)`.
    fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey>;

    /// Make a key from parts
    fn make_key(col: &Self::ColKey, row: &Self::RowKey) -> Self::Key;
}
