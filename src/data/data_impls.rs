// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Impls for data traits

use super::*;
use std::ops::{Deref, DerefMut};

impl<T: Debug> SharedData for [T] {
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}
impl<T: Debug> RecursivelyUpdatable for [T] {}
impl<T: Clone + Debug> ListData for [T] {
    type Key = usize;
    type Item = T;

    fn len(&self) -> usize {
        (*self).len()
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

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.iter().cloned().enumerate().take(limit).collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.iter()
            .cloned()
            .enumerate()
            .skip(start)
            .take(limit)
            .collect()
    }
}
impl<T: Clone + Debug> ListDataMut for [T] {
    fn set(&mut self, key: &Self::Key, item: Self::Item) {
        self[*key] = item;
    }
}

impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug> SharedData
    for std::collections::BTreeMap<K, T>
{
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}
impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug> RecursivelyUpdatable
    for std::collections::BTreeMap<K, T>
{
}
impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug> ListData for std::collections::BTreeMap<K, T> {
    type Key = K;
    type Item = T;

    fn len(&self) -> usize {
        (*self).len()
    }

    fn contains_key(&self, key: &Self::Key) -> bool {
        (*self).contains_key(key)
    }

    fn get_cloned(&self, key: &Self::Key) -> Option<Self::Item> {
        (*self).get(key).cloned()
    }

    fn update(&self, _: &Self::Key, _: Self::Item) -> Option<UpdateHandle> {
        // Note: plain BTreeMap does not support update, but SharedRc<..> does.
        None
    }

    fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.iter()
            .take(limit)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
        self.iter()
            .skip(start)
            .take(limit)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

// TODO(spec): implement using Deref; for now can't since it "might" conflict
// with a RefCell impl on a derived type downstream, according to the solver.
// impl<T: Deref + Debug> SingleData for T
// where
//     <T as Deref>::Target: SingleData,
macro_rules! impl_via_deref {
    ($t: ident: $derived:ty) => {
        impl<$t: SharedData + ?Sized> SharedData for $derived {
            fn update_handle(&self) -> Option<UpdateHandle> {
                self.deref().update_handle()
            }
            fn update_self(&self) -> Option<UpdateHandle> {
                self.deref().update_self()
            }
       }
        impl<$t: RecursivelyUpdatable + ?Sized> RecursivelyUpdatable for $derived {
            fn enable_recursive_updates(&self, mgr: &mut Manager) {
                self.deref().enable_recursive_updates(mgr);
            }
        }

        impl<$t: SingleData + ?Sized> SingleData for $derived {
            type Item = $t::Item;
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

            fn len(&self) -> usize {
                self.deref().len()
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

            fn iter_vec(&self, limit: usize) -> Vec<(Self::Key, Self::Item)> {
                self.deref().iter_vec(limit)
            }
            fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<(Self::Key, Self::Item)> {
                self.deref().iter_vec_from(start, limit)
            }
        }

        impl<$t: MatrixData + ?Sized> MatrixData for $derived {
            type ColKey = $t::ColKey;
            type RowKey = $t::RowKey;
            type Item = $t::Item;

            fn col_len(&self) -> usize {
                self.deref().col_len()
            }
            fn row_len(&self) -> usize {
                self.deref().row_len()
            }
            fn contains(&self, col: &Self::ColKey, row: &Self::RowKey) -> bool {
                self.deref().contains(col, row)
            }
            fn get_cloned(&self, col: &Self::ColKey, row: &Self::RowKey) -> Option<Self::Item> {
                self.deref().get_cloned(col, row)
            }

            fn update(
                &self,
                col: &Self::ColKey,
                row: &Self::RowKey,
                value: Self::Item,
            ) -> Option<UpdateHandle> {
                self.deref().update(col, row, value)
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
            fn set(&mut self, col: &Self::ColKey, row: &Self::RowKey, item: Self::Item) {
                self.deref_mut().set(col, row, item)
            }
        }
    };
    ($t: ident: $derived:ty, $($dd:ty),+) => {
        impl_via_deref_mut!($t: $derived);
        impl_via_deref_mut!($t: $($dd),+);
    };
}
impl_via_deref_mut!(T: &mut T, Box<T>);
