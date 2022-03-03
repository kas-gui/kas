// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects
//!
//! These traits are used for "view widgets", enabling views (and editing) over
//! shared data.
//!
//! Shared data must implement these traits:
//!
//! -   [`Updatable`]: used to expose the [`UpdateHandle`] on which widgets and
//!     other data may request updates; may also implement self-updates
//! -   [`UpdatableHandler`]: allows data updates from widget messages (or
//!     potentially from other message sources)

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

/// Shared (data) objects which may notify of updates
pub trait Updatable: Debug {
    /// Get any update handles used to notify of updates
    ///
    /// If the data supports updates through shared references (e.g. via an
    /// internal [`RefCell`]), then it should have an [`UpdateHandle`] for
    /// notifying other users of the data of the update. All [`UpdateHandle`]s
    /// used should be returned here. View widgets should check the data version
    /// and update their view when any of these [`UpdateHandle`]s is triggreed.
    fn update_handles(&self) -> Vec<UpdateHandle>;
}

/// Trait for data objects which can handle messages
pub trait UpdatableHandler<K, M>: Updatable {
    /// Update data, if supported
    ///
    /// This is optional and required only to support data updates through view
    /// widgets. If implemented, then [`Updatable::update_handle`] should
    /// return a copy of the same update handle.
    ///
    /// This method should return some [`UpdateHandle`] if the data was changed
    /// by this method, or `None` if nothing happened.
    ///
    /// This method takes only `&self`, thus probably [`RefCell`] will be used
    /// internally, alongside an [`UpdateHandle`].
    fn handle(&self, key: &K, msg: &M) -> Option<UpdateHandle>;
}

// TODO(spec): can we add this?
// impl<K, T> UpdatableHandler<K, VoidMsg> for T {
//     fn handle(&self, _: &K, msg: &VoidMsg) -> Option<UpdateHandle> {
//         match *msg {}
//     }
// }

impl<T: Debug> Updatable for [T] {
    fn update_handles(&self) -> Vec<UpdateHandle> {
        vec![]
    }
}
impl<T: Debug, M> UpdatableHandler<usize, M> for [T] {
    fn handle(&self, _: &usize, _: &M) -> Option<UpdateHandle> {
        None
    }
}

impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug> Updatable for std::collections::BTreeMap<K, T> {
    fn update_handles(&self) -> Vec<UpdateHandle> {
        vec![]
    }
}
impl<K: Ord + Eq + Clone + Debug, T: Clone + Debug, M> UpdatableHandler<K, M>
    for std::collections::BTreeMap<K, T>
{
    fn handle(&self, _: &K, _: &M) -> Option<UpdateHandle> {
        None
    }
}

macro_rules! impl_via_deref {
    ($t: ident: $derived:ty) => {
        impl<$t: Updatable + ?Sized> Updatable for $derived {
            fn update_handles(&self) -> Vec<UpdateHandle> {
                self.deref().update_handles()
            }
        }
        impl<K, M, $t: UpdatableHandler<K, M> + ?Sized> UpdatableHandler<K, M> for $derived {
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
