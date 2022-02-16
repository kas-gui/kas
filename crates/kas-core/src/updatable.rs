// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects
//!
//! These traits are used for "view widgets", enabling views (and editing) over
//! shared data.
//!
//! Shared data must implement the [`Updatable`] trait.

mod data_impls;
mod data_traits;
pub mod filter;
mod shared_rc;

#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;

pub use data_traits::{
    ListData, ListDataMut, MatrixData, MatrixDataMut, SingleData, SingleDataMut,
};
pub use shared_rc::SharedRc;

/// Shared (data) objects
pub trait Updatable<K, M>: Debug {
    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets.
    ///
    /// This method should return `true` if the data was changed.
    ///
    /// This method takes only `&self`, thus probably [`RefCell`] will be used
    /// internally.
    fn handle(&self, key: &K, msg: &M) -> bool;
}

// TODO(spec): can we add this?
// impl<K, T> Updatable<K, VoidMsg> for T {
//     fn handle(&self, _: &K, msg: &VoidMsg) -> bool {
//         match *msg {}
//     }
// }

impl<T: Debug, M> Updatable<usize, M> for [T] {
    fn handle(&self, _: &usize, _: &M) -> bool {
        false
    }
}

impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug, M> Updatable<K, M>
    for std::collections::BTreeMap<K, T>
{
    fn handle(&self, _: &K, _: &M) -> bool {
        false
    }
}

macro_rules! impl_via_deref {
    ($t: ident: $derived:ty) => {
        impl<K, M, $t: Updatable<K, M> + ?Sized> Updatable<K, M> for $derived {
            fn handle(&self, key: &K, msg: &M) -> bool {
                self.deref().handle(key, msg)
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
