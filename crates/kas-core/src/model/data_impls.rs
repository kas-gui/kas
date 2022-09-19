// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Impls for data traits

use super::*;
use crate::WidgetId;
use std::fmt::Debug;

macro_rules! impl_list_data {
    ($ty:ty) => {
        impl<T: Clone + Debug + 'static> SharedData for $ty {
            type Key = usize;
            type Item = T;
            type ItemRef<'b> = &'b T;

            fn version(&self) -> u64 {
                1
            }

            fn contains_key(&self, key: &Self::Key) -> bool {
                *key < self.len()
            }
            fn borrow(&self, key: &Self::Key) -> Option<Self::ItemRef<'_>> {
                self.get(*key)
            }
            fn get_cloned(&self, key: &usize) -> Option<Self::Item> {
                self.get(*key).cloned()
            }
        }
        impl<T: Clone + Debug + 'static> ListData for $ty {
            fn is_empty(&self) -> bool {
                (*self).is_empty()
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

            fn iter_vec(&self, limit: usize) -> Vec<Self::Key> {
                (0..limit.min((*self).len())).collect()
            }

            fn iter_vec_from(&self, start: usize, limit: usize) -> Vec<Self::Key> {
                let len = (*self).len();
                (start.min(len)..(start + limit).min(len)).collect()
            }
        }
    };
}

impl_list_data!([T]);
impl_list_data!(Vec<T>);
