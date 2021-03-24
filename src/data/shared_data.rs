// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use kas::event::{Manager, UpdateHandle};
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Shared (data) objects which may notify of updates
pub trait Updatable: Debug {
    /// Get an update handle, if any is used to notify of updates
    ///
    /// If the data supports updates through shared references (e.g. via an
    /// internal `RefCell`), then it should have an `UpdateHandle` for notifying
    /// other users of the data of the update, and return that here.
    /// Otherwise, this may simply return `None`.
    ///
    /// Users registering for updates on this handle should, if possible, also
    /// call [`RecursivelyUpdatable::enable_recursive_updates`].
    fn update_handle(&self) -> Option<UpdateHandle>;

    /// Update self from an update handle
    ///
    /// Data views which are themselves dependent on other shared data should
    /// register themselves for update via [`Manager::update_shared_data`].
    fn update_self(&self) -> Option<UpdateHandle> {
        None
    }
}

/// Recursive update support
///
/// All shared data types should also implement this trait. Only those which
/// require recursive updates need to provide a custom implementation of
/// `enable_recursive_updates`, and when they do it may only be possible to
/// implement this trait on `Rc<DataType>`.
//
// TODO(spec): implement this for all `Updatable` with a default impl? The cost
// is some non-functional impls, e.g. FilteredList<T, F> (which needs Rc<..>).
pub trait RecursivelyUpdatable: Updatable {
    /// Enable recursive updates on this object
    ///
    /// Some data objects (e.g. filters) are themselves dependent on another
    /// data object; this method allows such objects to register for updates on
    /// the underlying object. It should be called by any view over the data.
    ///
    /// The default implementation does nothing.
    fn enable_recursive_updates(&self, mgr: &mut Manager) {
        let _ = mgr;
    }
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

/// Bound over all other "updatable" traits
///
/// This is intended for usage as a bound where [`Updatable`],
/// [`RecursivelyUpdatable`] and [`UpdatableHandler`] implementations are all
/// required. It is automatically implemented when all these traits are.
pub trait UpdatableAll<K, M>: RecursivelyUpdatable + UpdatableHandler<K, M> {}
impl<K, M, T: RecursivelyUpdatable + UpdatableHandler<K, M>> UpdatableAll<K, M> for T {}
