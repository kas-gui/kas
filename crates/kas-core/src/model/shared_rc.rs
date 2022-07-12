// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets
//!
//! TODO: `SharedRc` makes the `sync-counter` example simpler, but most real
//! uses of shared data require custom impls anyway, so is this worth keeping?
//! If not, we can probably remove `ListDataMut` and other `*Mut` traits too.
//! Probably this question requires seeing more examples/applications to answer.

use crate::event::EventMgr;
use crate::event::UpdateId;
use crate::model::*;
use std::cell::{BorrowError, Ref, RefCell};
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

/// Wrapper for single-thread shared data
///
/// This is vaguely `Rc<RefCell<T>>`, but includes an [`UpdateId`] and a `u64`
/// version counter.
///
/// The wrapped value may be read via [`Self::borrow`], [`Self::try_borrow`] and
/// [`SingleData::get_cloned`].
///
/// The value may be set via [`SingleData::update`] and [`SingleDataMut::set`].
///
/// This wrapper type may be useful for simple shared data, but for more complex
/// uses a custom wrapper type may be required.
#[derive(Clone, Debug, Default)]
pub struct SharedRc<T: Debug>(Rc<(UpdateId, RefCell<(T, u64)>)>);

pub struct SharedRcRef<'a, T>(Ref<'a, (T, u64)>);
impl<'a, T> Deref for SharedRcRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.deref().0
    }
}

impl<T: Debug> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        let id = UpdateId::new();
        let data = RefCell::new((data, 1));
        SharedRc(Rc::new((id, data)))
    }

    /// Access update identifier
    ///
    /// Data updates via this [`SharedRc`] are triggered using this [`UpdateId`].
    pub fn id(&self) -> UpdateId {
        (self.0).0
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably
    /// borrowed.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    ///
    /// # Panics
    ///
    /// Panics if the value is currently mutably borrowed.
    /// For a non-panicking variant, use [`Self::try_borrow`].
    pub fn borrow(&self) -> SharedRcRef<T> {
        SharedRcRef((self.0).1.borrow())
    }

    /// Immutably borrows the wrapped value, returning an error if the value is currently mutably
    /// borrowed.
    ///
    /// The borrow lasts until the returned `Ref` exits scope. Multiple immutable borrows can be
    /// taken out at the same time.
    ///
    /// This is the non-panicking variant of [`Self::borrow`].
    pub fn try_borrow(&self) -> Result<SharedRcRef<T>, BorrowError> {
        (self.0).1.try_borrow().map(SharedRcRef)
    }
}

impl<T: Clone + Debug + 'static> SingleData for SharedRc<T> {
    type Item = T;

    fn version(&self) -> u64 {
        (self.0).1.borrow().1
    }

    fn get_cloned(&self) -> Self::Item {
        (self.0).1.borrow().0.to_owned()
    }

    fn update(&self, mgr: &mut EventMgr, value: Self::Item) {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = value;
        cell.1 += 1;
        mgr.update_all((self.0).0, 0);
    }
}
impl<T: Clone + Debug + 'static> SingleDataMut for SharedRc<T> {
    fn set(&mut self, value: Self::Item) {
        (self.0).1.borrow_mut().0 = value;
    }
}
