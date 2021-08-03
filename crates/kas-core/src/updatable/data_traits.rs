// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use crate::event::UpdateHandle;
#[allow(unused)] // doc links
use crate::updatable::Updatable;
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Trait for viewable single data items
// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
pub trait SingleData: Debug {
    /// Output type
    type Item: Clone;

    // TODO(gat): add get<'a>(&self) -> Self::ItemRef<'a> and get_mut

    /// Get data (clone)
    fn get_cloned(&self) -> Self::Item;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets. If implemented, then [`Updatable::update_handle`] should
    /// return a copy of the same update handle.
    ///
    /// Returns an [`UpdateHandle`] if an update occurred. Returns `None` if
    /// updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`SingleDataMut::set`]. The
    /// provider of this lowering should also provide an [`UpdateHandle`].
    fn update(&self, value: Self::Item) -> Option<UpdateHandle>;
}

/// Trait for writable single data items
pub trait SingleDataMut: SingleData {
    /// Set data, given a mutable (unique) reference
    ///
    /// It can be assumed that no synchronisation is required when a mutable
    /// reference can be obtained.
    fn set(&mut self, value: Self::Item);
}

/// Trait for viewable data lists
#[allow(clippy::len_without_is_empty)]
pub trait ListData: Debug {
    /// Key type
    type Key: Clone + Debug + PartialEq + Eq;

    /// Item type
    type Item: Clone;

    /// Number of data items available
    ///
    /// Note: users may assume this is `O(1)`.
    fn len(&self) -> usize;

    // TODO(gat): add get<'a>(&self) -> Self::ItemRef<'a> and get_mut

    /// Check whether a key has data
    fn contains_key(&self, key: &Self::Key) -> bool;

    /// Get data by key (clone)
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item>;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets. If implemented, then [`Updatable::update_handle`] should
    /// return a copy of the same update handle.
    ///
    /// Returns an [`UpdateHandle`] if an update occurred. Returns `None` if
    /// updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`ListDataMut::set`]. The
    /// provider of this lowering should also provide an [`UpdateHandle`].
    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle>;

    // TODO(gat): replace with an iterator
    /// Iterate over (key, value) pairs as a vec
    ///
    /// The result will be in deterministic implementation-defined order, with
    /// a length of `max(limit, data_len)` where `data_len` is the number of
    /// items available.
    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.iter_vec_from(0, limit)
    }

    /// Iterate over (key, value) pairs as a vec
    ///
    /// The result is the same as `self.iter_vec(start + limit).skip(start)`.
    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)>;
}

/// Trait for writable data lists
pub trait ListDataMut: ListData {
    /// Set data for an existing key
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

    /// Number of columns available
    ///
    /// Note: users may assume this is `O(1)`.
    fn col_len(&self) -> usize;

    /// Number of rows available
    ///
    /// Note: users may assume this is `O(1)`.
    fn row_len(&self) -> usize;

    /// Check whether an item with these keys exists
    fn contains(&self, key: &Self::Key) -> bool;

    /// Get data by key (clone)
    ///
    /// It is expected that this method succeeds when both keys are valid.
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item>;

    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets. If implemented, then [`Updatable::update_handle`] should
    /// return a copy of the same update handle.
    ///
    /// Returns an [`UpdateHandle`] if an update occurred. Returns `None` if
    /// updates are unsupported.
    ///
    /// This method takes only `&self`, thus some mechanism such as [`RefCell`]
    /// is required to obtain `&mut` and lower to [`ListDataMut::set`]. The
    /// provider of this lowering should also provide an [`UpdateHandle`].
    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle>;

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
    fn set(&mut self, key: &Self::Key, item: Self::Item);
}
