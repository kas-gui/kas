// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `SharedArc` data type

#[allow(unused)] use crate::event::Event;
use crate::event::{EventMgr, UpdateId};
use crate::model::*;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

/// Wrapper for multi-threaded shared data
///
/// This is vaguely `Arc<Mutex<T>>`, but includes an [`UpdateId`] and a `u64`
/// version counter. Its main utility is that it implements the [`SharedData`]
/// and [`SharedDataMut`] traits (with associated type `Key = ()`).
#[derive(Clone, Debug)]
pub struct SharedArc<T: Debug>(Arc<(UpdateId, Mutex<(T, u64)>)>);

impl<T: Debug + Default> Default for SharedArc<T> {
    fn default() -> Self {
        SharedArc(Arc::new((UpdateId::new(), Default::default())))
    }
}

/// A borrowed reference
pub struct SharedArcRef<'a, T>(MutexGuard<'a, (T, u64)>);
impl<'a, T> std::borrow::Borrow<T> for SharedArcRef<'a, T> {
    fn borrow(&self) -> &T {
        &self.0.deref().0
    }
}
impl<'a, T> Deref for SharedArcRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.deref().0
    }
}

/// A mutably borrowed reference
//
// Note: this is identical to SharedArcRef other than the trait impls. We cannot
// allow SharedData::borrow to return a type supporting mutable access!
pub struct SharedArcRefMut<'a, T>(MutexGuard<'a, (T, u64)>);
impl<'a, T> std::borrow::Borrow<T> for SharedArcRefMut<'a, T> {
    fn borrow(&self) -> &T {
        &self.0.deref().0
    }
}
impl<'a, T> std::borrow::BorrowMut<T> for SharedArcRefMut<'a, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0.deref_mut().0
    }
}
impl<'a, T> Deref for SharedArcRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.deref().0
    }
}
impl<'a, T> DerefMut for SharedArcRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0.deref_mut().0
    }
}

impl<T: Debug> SharedArc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        let id = UpdateId::new();
        let data = Mutex::new((data, 1));
        SharedArc(Arc::new((id, data)))
    }

    /// Access update identifier
    ///
    /// Data updates via this [`SharedArc`] are triggered using this [`UpdateId`].
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
        (self.0).1.lock().unwrap().1
    }

    /// Borrow an item
    ///
    /// May panic (see [`Mutex::lock`]).
    pub fn borrow(&self) -> SharedArcRef<'_, T> {
        SharedArcRef((self.0).1.lock().unwrap())
    }

    /// Mutably borrow an item
    ///
    /// This notifies users of a data update by calling [`EventMgr::update_all`]
    /// and incrementing the number returned by [`Self::version`].
    ///
    /// May panic (see [`Mutex::lock`]).
    pub fn borrow_mut(&self, mgr: &mut EventMgr) -> SharedArcRefMut<'_, T> {
        mgr.update_with_id((self.0).0, 0);
        let mut inner = (self.0).1.lock().unwrap();
        inner.1 += 1;
        SharedArcRefMut(inner)
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

impl<T: Clone + Debug> SharedArc<T> {
    /// Get a clone of the stored item
    #[inline]
    pub fn get_cloned(&self) -> T {
        self.borrow().deref().clone()
    }
}

impl<T: Clone + Debug + 'static> SharedData for SharedArc<T> {
    type Key = ();
    type Item = T;
    type ItemRef<'b> = SharedArcRef<'b, T>;

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
impl<T: Clone + Debug + 'static> SharedDataMut for SharedArc<T> {
    type ItemRefMut<'b> = SharedArcRefMut<'b, T>
    where
        Self: 'b;

    #[inline]
    fn borrow_mut(&self, mgr: &mut EventMgr, _: &Self::Key) -> Option<Self::ItemRefMut<'_>> {
        Some(self.borrow_mut(mgr))
    }
}