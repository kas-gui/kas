// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `SharedRc` data type

use crate::event::EventMgr;
use crate::event::UpdateId;
use crate::model::*;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

/// Wrapper for single-thread shared data
///
/// This is vaguely `Rc<RefCell<T>>`, but includes an [`UpdateId`] and a `u64`
/// version counter. Its main utility is that it implements the [`SharedData`]
/// and [`SharedDataMut`] traits (with associated type `Key = ()`).
#[derive(Clone, Debug)]
pub struct SharedRc<T: Debug>(Rc<(UpdateId, RefCell<(T, u64)>)>);

impl<T: Debug + Default> Default for SharedRc<T> {
    fn default() -> Self {
        SharedRc(Rc::new((UpdateId::new(), Default::default())))
    }
}

/// A borrowed reference
pub struct SharedRcRef<'a, T>(Ref<'a, T>);
impl<'a, T> std::borrow::Borrow<T> for SharedRcRef<'a, T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}
impl<'a, T> Deref for SharedRcRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// A mutably borrowed reference
pub struct SharedRcRefMut<'a, T>(RefMut<'a, T>);
impl<'a, T> std::borrow::Borrow<T> for SharedRcRefMut<'a, T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}
impl<'a, T> std::borrow::BorrowMut<T> for SharedRcRefMut<'a, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
impl<'a, T> Deref for SharedRcRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<'a, T> DerefMut for SharedRcRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
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
    /// This is the "update identifier" payload of the [`Event::Update`] sent to
    /// widgets when this data is modified.
    pub fn id(&self) -> UpdateId {
        (self.0).0
    }

    /// Get the data version
    ///
    /// The version is increased on change and may be used to detect when views
    /// over the data need to be refreshed. The initial version number must be
    /// at least 1 (allowing 0 to represent an uninitialized state).
    ///
    /// Whenever the data is updated, [`Event::Update`] must be sent via
    /// [`EventMgr::update_all`] to notify other users of this data of the
    /// update.
    pub fn version(&self) -> u64 {
        (self.0).1.borrow().1
    }

    /// Borrow an item
    ///
    /// May panic (see [`RefCell::borrow`]).
    pub fn borrow(&self) -> SharedRcRef<'_, T> {
        SharedRcRef(Ref::map((self.0).1.borrow(), |tuple| &tuple.0))
    }

    /// Mutably borrow an item
    ///
    /// This notifies users of a data update by calling [`EventMgr::update_all`]
    /// and incrementing the number returned by [`Self::version`].
    ///
    /// May panic (see [`RefCell::borrow_mut`]).
    pub fn borrow_mut(&self, mgr: &mut EventMgr) -> SharedRcRefMut<'_, T> {
        mgr.update_with_id((self.0).0, 0);
        let mut cell = (self.0).1.borrow_mut();
        cell.1 += 1;
        SharedRcRefMut(RefMut::map(cell, |tuple| &mut tuple.0))
    }

    /// Set an item
    ///
    /// This notifies users of a data update by calling [`EventMgr::update_all`]
    /// and incrementing the number returned by [`Self::version`].
    #[inline]
    pub fn set(&self, mgr: &mut EventMgr, item: T) {
        *self.borrow_mut(mgr) = item;
    }
}

impl<T: Clone + Debug> SharedRc<T> {
    /// Get a clone of the stored item
    #[inline]
    pub fn get_cloned(&self) -> T {
        self.borrow().deref().clone()
    }
}

impl<T: Clone + Debug + 'static> SharedData for SharedRc<T> {
    type Key = ();
    type Item = T;
    type ItemRef<'b> = SharedRcRef<'b, T>;

    #[inline]
    fn version(&self) -> u64 {
        self.version()
    }

    #[inline]
    fn contains_key(&self, _: &()) -> bool {
        true
    }
    #[inline]
    fn borrow(&self, _: &Self::Key) -> Option<Self::ItemRef<'_>> {
        Some(self.borrow())
    }
}
impl<T: Clone + Debug + 'static> SharedDataMut for SharedRc<T> {
    type ItemRefMut<'b> = SharedRcRefMut<'b, T>
    where
        Self: 'b;

    #[inline]
    fn borrow_mut(&self, mgr: &mut EventMgr, _: &Self::Key) -> Option<Self::ItemRefMut<'_>> {
        Some(self.borrow_mut(mgr))
    }
}
