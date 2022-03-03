// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects
//!
//! These traits are used for "view widgets", enabling views (and editing) over
//! shared data.
//!
//! Shared data must implement [`Updatable`] to allows data updates
//! from widget messages (or potentially from other message sources).

mod data_impls;
mod data_traits;
pub mod filter;
mod shared_rc;

use crate::event::UpdateHandle;
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;

pub use data_traits::{
    ListData, ListDataMut, MatrixData, MatrixDataMut, SingleData, SingleDataMut,
};
pub use shared_rc::SharedRc;

/// Trait for data objects which can handle messages
pub trait Updatable<K, M> {
    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets.
    ///
    /// This method should return some [`UpdateHandle`] if the data was changed
    /// by this method, or `None` if nothing happened.
    ///
    /// This method takes only `&self`, thus probably [`RefCell`] will be used
    /// internally, alongside an [`UpdateHandle`].
    fn handle(&self, key: &K, msg: &M) -> Option<UpdateHandle>;
}

// TODO(spec): can we add this?
// impl<K, T> Updatable<K, VoidMsg> for T {
//     fn handle(&self, _: &K, msg: &VoidMsg) -> Option<UpdateHandle> {
//         match *msg {}
//     }
// }

impl<T: Debug, M> Updatable<usize, M> for [T] {
    fn handle(&self, _: &usize, _: &M) -> Option<UpdateHandle> {
        None
    }
}

impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug, M> Updatable<K, M>
    for std::collections::BTreeMap<K, T>
{
    fn handle(&self, _: &K, _: &M) -> Option<UpdateHandle> {
        None
    }
}

macro_rules! impl_via_deref {
    ($t: ident: $derived:ty) => {
        impl<K, M, $t: Updatable<K, M> + ?Sized> Updatable<K, M> for $derived {
            fn handle(&self, key: &K, msg: &M) -> Option<UpdateHandle> {
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
