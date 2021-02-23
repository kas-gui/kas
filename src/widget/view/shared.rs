// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets

use super::{ListData, SingleData, SingleDataMut};
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

/// Wrapper for shared constant data
///
/// This may be useful with static data, e.g. `[&'static str]`.
#[derive(Clone, Debug, Default)]
pub struct SharedConst<T: Debug + 'static + ?Sized>(T);

impl<T: Debug + 'static> SharedConst<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedConst(data)
    }
}

impl<T: Debug + 'static> From<T> for SharedConst<T> {
    fn from(data: T) -> Self {
        SharedConst(data)
    }
}

impl<T: Debug + 'static + ?Sized> From<&T> for &SharedConst<T> {
    fn from(data: &T) -> Self {
        // SAFETY: SharedConst<T> is a thin wrapper around T
        unsafe { &*(data as *const T as *const SharedConst<T>) }
    }
}

impl<T: Clone + Debug + 'static + ?Sized> SingleData for SharedConst<T> {
    type Item = T;

    fn get_cloned(&self) -> Self::Item {
        self.0.clone()
    }
}

impl<T: ListData + 'static + ?Sized> ListData for SharedConst<T> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.0.get_cloned(key)
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.0.iter_vec(limit)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.0.iter_vec_from(start, limit)
    }
}

/// Wrapper for single-thread shared data
#[derive(Clone, Debug)]
pub struct SharedRc<T: Debug> {
    handle: UpdateHandle,
    data: Rc<RefCell<T>>,
}

impl<T: Default + Debug> Default for SharedRc<T> {
    fn default() -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Default::default(),
        }
    }
}

impl<T: Debug> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Rc::new(RefCell::new(data)),
        }
    }
}

impl<T: Clone + Debug> SingleData for SharedRc<T> {
    type Item = T;

    fn get_cloned(&self) -> Self::Item {
        self.data.borrow().to_owned()
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
impl<T: Clone + Debug> SingleDataMut for SharedRc<T> {
    fn set(&self, value: Self::Item) -> UpdateHandle {
        *self.data.borrow_mut() = value;
        self.handle
    }
}

impl<T: ListData> ListData for SharedRc<T> {
    type Key = T::Key;
    type Item = T::Item;

    fn len(&self) -> usize {
        self.data.borrow().len()
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        self.data.borrow().get_cloned(key)
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.data.borrow().iter_vec(limit)
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.data.borrow().iter_vec_from(start, limit)
    }

    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}
