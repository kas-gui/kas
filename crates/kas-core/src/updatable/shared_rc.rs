// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets
//!
//! TODO: `SharedRc` makes the `sync-counter` example simpler, but most real
//! uses of shared data require custom impls anyway, so is this worth keeping?
//! If not, we can probably remove `ListDataMut` and other `*Mut` traits too.
//! Probably this question requires seeing more examples/applications to answer.

#[allow(unused)]
use crate::event::EventMgr;
use crate::event::UpdateHandle;
use crate::updatable::*;
use crate::WidgetId;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Wrapper for single-thread shared data
///
/// This wrapper adds an [`UpdateHandle`] and implements the [`Updatable`] and
/// [`UpdatableHandler`] traits (the latter with a dummy implementation —
/// if you need custom handlers you will need your own shared data type).
#[derive(Clone, Debug, Default)]
pub struct SharedRc<T: Debug>(Rc<(UpdateHandle, RefCell<(T, u64)>)>);

impl<T: Debug> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        let handle = UpdateHandle::new();
        let data = RefCell::new((data, 0));
        SharedRc(Rc::new((handle, data)))
    }
}
impl<T: Debug> Updatable for SharedRc<T> {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some((self.0).0)
    }
}

impl<T: Clone + Debug, K, M> UpdatableHandler<K, M> for SharedRc<T> {
    fn handle(&self, _: &K, _: &M) -> Option<UpdateHandle> {
        None
    }
}

impl<T: Clone + Debug> SingleData for SharedRc<T> {
    type Item = T;

    fn version(&self) -> u64 {
        (self.0).1.borrow().1
    }

    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().0.to_owned()
    }

    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 += 1;
        Some((self.0).0)
    }
}
impl<T: Clone + Debug> SingleDataMut for SharedRc<T> {
    fn set(&mut self, value: Self::Item) {
        (self.0).1.borrow_mut().0 = value;
    }
}

impl<T: ListDataMut> ListData for SharedRc<T> {
    type Key = T::Key;
    type Item = T::Item;

    fn version(&self) -> u64 {
        let cell = (self.0).1.borrow();
        cell.0.version() + cell.1
    }

    fn len(&self) -> usize {
        (self.0).1.borrow().0.len()
    }

    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        (self.0).1.borrow().0.make_id(parent, key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        (self.0).1.borrow().0.reconstruct_key(parent, child)
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        (self.0).1.borrow().0.contains_key(key)
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        (self.0).1.borrow().0.get_cloned(key)
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        let mut cell = (self.0).1.borrow_mut();
        cell.0.set(key, value);
        cell.1 += 1;
        Some((self.0).0)
    }

    fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
        (self.0).1.borrow().0.iter_vec(limit)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
        (self.0).1.borrow().0.iter_vec_from(start, limit)
    }
}
impl<T: ListDataMut> ListDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        (self.0).1.borrow_mut().0.set(key, item);
    }
}

impl<T: MatrixDataMut> MatrixData for SharedRc<T> {
    type ColKey = T::ColKey;
    type RowKey = T::RowKey;
    type Key = T::Key;
    type Item = T::Item;

    fn version(&self) -> u64 {
        let cell = (self.0).1.borrow();
        cell.0.version() + cell.1
    }

    fn len(&self) -> (usize, usize) {
        (self.0).1.borrow().0.len()
    }

    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        (self.0).1.borrow().0.make_id(parent, key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        (self.0).1.borrow().0.reconstruct_key(parent, child)
    }

    fn contains(&self, key: &Self::Key) -> bool {
        (self.0).1.borrow().0.contains(key)
    }
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        (self.0).1.borrow().0.get_cloned(key)
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        let mut cell = (self.0).1.borrow_mut();
        cell.0.set(key, value);
        cell.1 += 1;
        Some((self.0).0)
    }

    fn col_iter_vec(&self, limit: usize) -> Vec<Self::ColKey> {
        (self.0).1.borrow().0.col_iter_vec(limit)
    }
    fn col_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::ColKey> {
        (self.0).1.borrow().0.col_iter_vec_from(start, limit)
    }

    fn row_iter_vec(&self, limit: usize) -> Vec<Self::RowKey> {
        (self.0).1.borrow().0.row_iter_vec(limit)
    }
    fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey> {
        (self.0).1.borrow().0.row_iter_vec_from(start, limit)
    }

    fn make_key(row: &Self::RowKey, col: &Self::ColKey) -> Self::Key {
        T::make_key(row, col)
    }
}
impl<T: MatrixDataMut> MatrixDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        (self.0).1.borrow_mut().0.set(key, item);
    }
}
