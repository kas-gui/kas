// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Impls for data traits

use super::*;
use crate::event::UpdateHandle;
use crate::WidgetId;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

impl<T: Clone + Debug> ListData for [T] {
    type Key = usize;
    type Item = T;

    fn version(&self) -> u64 {
        0
    }

    fn len(&self) -> usize {
        (*self).len()
    }

    fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
        parent.make_child(*key)
    }
    fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
        child.next_key_after(parent)
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        *key < self.len()
    }

    fn get_cloned(&self, key: &usize) -> Option<Self::Item> {
        self.get(*key).cloned()
    }

    fn update(&self, _: &Self::Key, _: Self::Item) -> Option<UpdateHandle> {
        // Note: plain [T] does not support update, but SharedRc<[T]> does.
        None
    }

    fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
        (0..limit.min((*self).len())).collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
        let len = (*self).len();
        (start.min(len)..(start + limit).min(len)).collect()
    }
}
impl<T: Clone + Debug> ListDataMut for [T] {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        self[*key] = item;
    }
}

// TODO(spec): implement using Deref; for now can't since it "might" conflict
// with a RefCell impl on a derived type downstream, according to the solver.
// impl<T: Deref + Debug> SingleData for T
// where
//     <T as Deref>::Target: SingleData,
macro_rules! impl_via_deref {
    ($t: ident: $derived:ty) => {
        impl<$t: SingleData + ?Sized> SingleData for $derived {
            type Item = $t::Item;
            fn version(&self) -> u64 {
                self.deref().version()
            }
            fn get_cloned(&self) -> Self::Item {
                self.deref().get_cloned()
            }
            fn update(&self, value: Self::Item) -> Option<UpdateHandle> {
                self.deref().update(value)
            }
        }

        impl<$t: ListData + ?Sized> ListData for $derived {
            type Key = $t::Key;
            type Item = $t::Item;

            fn version(&self) -> u64 {
                self.deref().version()
            }

            fn len(&self) -> usize {
                self.deref().len()
            }
            fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
                self.deref().make_id(parent, key)
            }
            fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
                self.deref().reconstruct_key(parent, child)
            }

            fn contains_key(&self, key: &Self::Key) -> bool {
                self.deref().contains_key(key)
            }
            fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
                self.deref().get_cloned(key)
            }

            fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
                self.deref().update(key, value)
            }

            fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
                self.deref().iter_vec(limit)
            }
            fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
                self.deref().iter_vec_from(start, limit)
            }
        }

        impl<$t: MatrixData + ?Sized> MatrixData for $derived {
            type ColKey = $t::ColKey;
            type RowKey = $t::RowKey;
            type Key = $t::Key;
            type Item = $t::Item;

            fn version(&self) -> u64 {
                self.deref().version()
            }

            fn len(&self) -> (usize, usize) {
                self.deref().len()
            }
            fn make_id(&self, parent: &WidgetId, key: &Self::Key) -> WidgetId {
                self.deref().make_id(parent, key)
            }
            fn reconstruct_key(&self, parent: &WidgetId, child: &WidgetId) -> Option<Self::Key> {
                self.deref().reconstruct_key(parent, child)
            }

            fn contains(&self, key: &Self::Key) -> bool {
                self.deref().contains(key)
            }
            fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
                self.deref().get_cloned(key)
            }

            fn update(&self, key: &Self::Key, value: Self::Item) -> Option<UpdateHandle> {
                self.deref().update(key, value)
            }

            fn col_iter_vec(&self, limit: usize) -> Vec<Self::ColKey> {
                self.deref().col_iter_vec(limit)
            }
            fn col_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::ColKey> {
                self.deref().col_iter_vec_from(start, limit)
            }

            fn row_iter_vec(&self, limit: usize) -> Vec<Self::RowKey> {
                self.deref().row_iter_vec(limit)
            }
            fn row_iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::RowKey> {
                self.deref().row_iter_vec_from(start, limit)
            }

            fn make_key(row: &Self::RowKey, col: &Self::ColKey) -> Self::Key {
                <$t>::make_key(row, col)
            }
        }
    };
    ($t: ident: $derived:ty, $($dd:ty),+) => {
        impl_via_deref!($t: $derived);
        impl_via_deref!($t: $($dd),+);
    };
}
impl_via_deref!(T: &T, &mut T);
impl_via_deref!(T: std::rc::Rc<T>, std::sync::Arc<T>, Box<T>);

macro_rules! impl_via_deref_mut {
    ($t: ident: $derived:ty) => {
        impl<$t: SingleDataMut + ?Sized> SingleDataMut for $derived {
            fn set(&mut self, value: Self::Item) {
                self.deref_mut().set(value)
            }
        }
        impl<$t: ListDataMut + ?Sized> ListDataMut for $derived {
            fn set(&mut self, key: &Self::Key, item: Self::Item) {
                self.deref_mut().set(key, item)
            }
        }
        impl<$t: MatrixDataMut + ?Sized> MatrixDataMut for $derived {
            fn set(&mut self, key: &Self::Key, item: Self::Item) {
                self.deref_mut().set(key, item)
            }
        }
    };
    ($t: ident: $derived:ty, $($dd:ty),+) => {
        impl_via_deref_mut!($t: $derived);
        impl_via_deref_mut!($t: $($dd),+);
    };
}
impl_via_deref_mut!(T: &mut T, Box<T>);
