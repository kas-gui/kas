// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use crate::WidgetId;
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Trait for viewable single data items
// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
pub trait SingleData: Debug {
    /// Output type
    type Item: Clone;

    /// Get the data version
    ///
    /// Views over shared data must check the data's `version` when drawn,
    /// comparing to a local cached version and updating the view if out-dated.
    ///
    /// Data structures may update themselves when `version` is called (by using
    /// an internal [`RefCell`], as required to support [`Self::update`]).
    ///
    /// The initial version number must be at least 1 (allowing 0 to represent
    /// an uninitialized state). Each modification of the data structure must
    /// increase the version number (allowing change detection).
    fn version(&self) -> u64;

    // TODO(gat): add get<'a>(&self) -> Self::ItemRef<'a> and get_mut

    /// Get data (clone)
    fn get_cloned(&self) -> Self::Item;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets.
    ///
    /// Updates the [`Self::version`] number and returns `true` if
    /// an update occurred. Returns `false` if updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`SingleDataMut::set`].
    fn update(&self, value: Self::Item) -> bool;
}

/// Trait for writable single data items
pub trait SingleDataMut: SingleData {
    /// Set data, given a mutable (unique) reference
    ///
    /// It can be assumed that no synchronisation is required when a mutable
    /// reference can be obtained. The `version` number need not be affected.
    fn set(&mut self, value: Self::Item);
}

/// Trait for viewable data lists
#[allow(clippy::len_without_is_empty)]
pub trait ListData: Debug {
    /// Key type
    type Key: Clone + Debug + PartialEq + Eq;

    /// Item type
    type Item: Clone;

    /// Get the data version
    ///
    /// Views over shared data must check the data's `version` when drawn,
    /// comparing to a local cached version and updating the view if out-dated.
    ///
    /// Data structures may update themselves when `version` is called (by using
    /// an internal [`RefCell`], as required to support [`Self::update`]).
    ///
    /// The initial version number must be at least 1 (allowing 0 to represent
    /// an uninitialized state). Each modification of the data structure must
    /// increase the version number (allowing change detection).
    fn version(&self) -> u64;

    /// Number of data items available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> usize;

    /// Make a [`WidgetId`] for a key
    ///
    /// The `parent` identifier is used as a reference.
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId;

    /// Reconstruct a key from a [`WidgetId`]
    ///
    /// The `parent` identifier is used as a reference.
    ///
    /// If the `child` identifier is one returned by [`Self::make_id`] for the
    /// same `parent`, *or descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key>;

    // TODO(gat): add get<'a>(&self) -> Self::ItemRef<'a> and get_mut

    /// Check whether a key has data
    fn contains_key(&self, key: &Self::Key) -> bool;

    /// Get data by key (clone)
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item>;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets.
    ///
    /// Updates the [`Self::version`] number and returns `true` if
    /// an update occurred. Returns `false` if updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`ListDataMut::set`].
    fn update(&self, key: &Self::Key, value: Self::Item) -> bool;

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

/// Trait for writable data lists
pub trait ListDataMut: ListData {
    /// Set data for an existing key
    ///
    /// It can be assumed that no synchronisation is required when a mutable
    /// reference can be obtained. The `version` number need not be affected.
    fn set(&mut self, key: &Self::Key, item: Self::Item);
}

/// Trait for viewable data matrices
///
/// Data matrices are a kind of table where each cell has the same type.
pub trait MatrixData: Debug {
    /// Column key type
    type ColKey: Clone + Debug + PartialEq + Eq;
    /// Row key type
    type RowKey: Clone + Debug + PartialEq + Eq;
    /// Full key type
    type Key: Clone + Debug + PartialEq + Eq;
    /// Item type
    type Item: Clone;

    /// Get the data version
    ///
    /// Views over shared data must check the data's `version` when drawn,
    /// comparing to a local cached version and updating the view if out-dated.
    ///
    /// Data structures may update themselves when `version` is called (by using
    /// an internal [`RefCell`], as required to support [`Self::update`]).
    ///
    /// The initial version number must be at least 1 (allowing 0 to represent
    /// an uninitialized state). Each modification of the data structure must
    /// increase the version number (allowing change detection).
    fn version(&self) -> u64;

    /// No data is available
    fn is_empty(&self) -> bool;

    /// Number of `(cols, rows)` available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> (usize, usize);

    /// Make a [`WidgetId`] for a key
    ///
    /// The `parent` identifier is used as a reference.
    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId;

    /// Reconstruct a key from a [`WidgetId`]
    ///
    /// The `parent` identifier is used as a reference.
    ///
    /// If the `child` identifier is one returned by [`Self::make_id`] for the
    /// same `parent`, *or descended from that*, this should return a copy of
    /// the `key` passed to `make_id`.
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key>;

    /// Check whether an item with these keys exists
    fn contains(&self, key: &Self::Key) -> bool;

    /// Get data by key (clone)
    ///
    /// It is expected that this method succeeds when both keys are valid.
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item>;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets.
    ///
    /// Updates the [`Self::version`] number and returns `true` if
    /// an update occurred. Returns `false` if updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`ListDataMut::set`].
    fn update(&self, key: &Self::Key, value: Self::Item) -> bool;

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

/// Trait for writable data matrices
pub trait MatrixDataMut: MatrixData {
    /// Set data for an existing cell
    ///
    /// It can be assumed that no synchronisation is required when a mutable
    /// reference can be obtained. The `version` number need not be affected.
    fn set(&mut self, key: &Self::Key, item: Self::Item);
}
