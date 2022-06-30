// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Impls for data traits

use super::*;
use crate::event::EventMgr;
use crate::WidgetId;
use std::fmt::Debug;

macro_rules! impl_list_data {
    ($ty:ty) => {
        impl<T: Clone + Debug + 'static> ListData for $ty {
            type Key = usize;
            type Item = T;

            fn version(&self) -> u64 {
                1
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

            fn update(&self, _: &mut EventMgr, _: &Self::Key, _: Self::Item) {
                // Note: plain [T] does not support update, but SharedRc<[T]> does.
            }

            fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
                (0..limit.min((*self).len())).collect()
            }

            fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
                let len = (*self).len();
                (start.min(len)..(start + limit).min(len)).collect()
            }
        }
        impl<T: Clone + Debug + 'static> ListDataMut for $ty {
            fn set(&mut self, key: &Self::Key, item: Self::Item) {
                self[*key] = item;
            }
        }
    };
}

impl_list_data!([T]);
impl_list_data!(Vec<T>);
