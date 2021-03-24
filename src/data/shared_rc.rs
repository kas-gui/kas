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

use kas::data::*;
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Wrapper for single-thread shared data
///
/// This wrapper adds an [`UpdateHandle`] and implements the [`Updatable`],
/// [`RecursivelyUpdatable`] and [`UpdatableHandler`] traits (the latter two
/// with dummy implementations — if you need custom handlers you will need your
/// own shared data type).
#[derive(Clone, Debug)]
pub struct SharedRc<T: Debug>(Rc<(UpdateHandle, RefCell<T>)>);

impl<T: Default + Debug> Default for SharedRc<T> {
    fn default() -> Self {
        let handle = UpdateHandle::new();
        let data = Default::default();
        SharedRc(Rc::new((handle, data)))
    }
}

impl<T: Debug> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        let handle = UpdateHandle::new();
        let data = RefCell::new(data);
        SharedRc(Rc::new((handle, data)))
    }
}
impl<T: Debug> Updatable for SharedRc<T> {
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some((self.0).0)
    }
}
impl<T: Debug> RecursivelyUpdatable for SharedRc<T> {}

impl<T: Clone + Debug, K, M> UpdatableHandler<K, M> for SharedRc<T> {
    fn handle(&self, _: &K, _: &M) -> Option<UpdateHandle> {
        None
    }
}

impl<T: Clone + Debug> SingleData for SharedRc<T> {
    type Item = T;

    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().to_owned()
    }

    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        *(self.0).1.borrow_mut() = value;
        Some((self.0).0)
    }
}
impl<T: Clone + Debug> SingleDataMut for SharedRc<T> {
    fn set(&mut self, value: Self::Item) {
        *(self.0).1.borrow_mut() = value;
    }
}

impl<T: ListDataMut> ListData for SharedRc<T> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        (self.0).1.borrow().len()
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        (self.0).1.borrow().contains_key(key)
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        (self.0).1.borrow().get_cloned(key)
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        (self.0).1.borrow_mut().set(key, value);
        Some((self.0).0)
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (self.0).1.borrow().iter_vec(limit)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        (self.0).1.borrow().iter_vec_from(start, limit)
    }
}
impl<T: ListDataMut> ListDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        (self.0).1.borrow_mut().set(key, item);
    }
}

impl<T: MatrixData> MatrixData for SharedRc<T> {
    type ColKey = T::ColKey;
    type RowKey = T::RowKey;
    type Key = T::Key;
    type Item = T::Item;

    fn col_len(&self) -> usize {
        (self.0).1.borrow().col_len()
    }
    fn row_len(&self) -> usize {
        (self.0).1.borrow().row_len()
    }
    fn contains(&self, key: &Self::Key) -> bool {
        (self.0).1.borrow().contains(key)
    }
    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        (self.0).1.borrow().get_cloned(key)
    }

    fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
        (self.0).1.borrow().update(key, value)
    }

    fn col_iter_vec(&self, limit: usize) -> Vec<Self::ColKey> {
        (self.0).1.borrow().col_iter_vec(limit)
    }
    fn col_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::ColKey> {
        (self.0).1.borrow().col_iter_vec_from(start, limit)
    }

    fn row_iter_vec(&self, limit: usize) -> Vec<Self::RowKey> {
        (self.0).1.borrow().row_iter_vec(limit)
    }
    fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey> {
        (self.0).1.borrow().row_iter_vec_from(start, limit)
    }

    fn make_key(col: &Self::ColKey, row: &Self::RowKey) -> Self::Key {
        T::make_key(col, row)
    }
}
impl<T: MatrixDataMut> MatrixDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        (self.0).1.borrow_mut().set(key, item);
    }
}
