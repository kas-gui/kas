// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets

use super::{ListData, ListDataMut, SingleData, SingleDataMut};
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Wrapper for single-thread shared data
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

impl<T: Clone + Debug> SingleData for SharedRc<T> {
    type Item = T;

    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().to_owned()
    }

    fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
        *(self.0).1.borrow_mut() = value;
        Some((self.0).0)
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
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

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some((self.0).0)
    }
}
impl<T: ListDataMut> ListDataMut for SharedRc<T> {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        (self.0).1.borrow_mut().set(key, item);
    }
}
