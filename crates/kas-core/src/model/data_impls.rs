// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Impls for data traits

use super::*;
use std::fmt::Debug;

macro_rules! impl_list_data {
    ($ty:ty) => {
        impl<T: Clone + Debug + 'static> SharedData for $ty {
            type Key = usize;
            type Item = T;
            type ItemRef<'b> = &'b T;

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
            type KeyIter<'b> = std::ops::Range<usize>;

            fn is_empty(&self) -> bool {
                (*self).is_empty()
            }

            fn len(&self) -> usize {
                (*self).len()
            }

            fn iter_from(&self, start: usize, limit: usize) -> Self::KeyIter<'_> {
                let len = (*self).len();
                start.min(len)..(start + limit).min(len)
            }
        }
    };
}

impl_list_data!([T]);
impl_list_data!(Vec<T>);
