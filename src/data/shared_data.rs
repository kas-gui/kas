// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Traits for shared data objects

use kas::event::{Manager, UpdateHandle};
#[allow(unused)] // doc links
use std::cell::RefCell;
use std::fmt::Debug;

/// Shared data which may notify of updates
pub trait SharedData: Debug {
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
pub trait RecursivelyUpdatable: Debug {
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
