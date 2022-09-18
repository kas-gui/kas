// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! `SharedArc` data type

use crate::event::EventMgr;
use crate::event::UpdateId;
use crate::model::*;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

/// Wrapper for multi-threaded shared data
///
/// This is vaguely `Arc<Mutex<T>>`, but includes an [`UpdateId`] and a `u64`
/// version counter.
///
/// The wrapped value may be read via [`Self::borrow`] and
/// [`SharedData::get_cloned`].
///
/// The value may be set via [`SharedData::update`].
///
/// This wrapper type may be useful for simple shared data, but for more complex
/// uses a custom wrapper type may be required.
#[derive(Clone, Debug)]
pub struct SharedArc<T: Debug>(Arc<(UpdateId, Mutex<(T, u64)>)>);

impl<T: Debug + Default> Default for SharedArc<T> {
    fn default() -> Self {
        SharedArc(Arc::new((UpdateId::new(), Default::default())))
    }
}

/// A borrowed reference
pub struct SharedArcRef<'a, T>(MutexGuard<'a, (T, u64)>);
impl<'a, T: Clone> MyBorrow<T> for SharedArcRef<'a, T> {
    fn as_ref(&self) -> &T {
        &self.0.deref().0
    }
}

/// A mutably borrowed reference
pub struct SharedArcRefMut<'a, T>(MutexGuard<'a, (T, u64)>);
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

    /// Mutably borrows the wrapped value, notifying other users of an update
    ///
    /// Internally this uses [`Mutex::lock`]; see its documentation regarding possible errors.
    ///
    /// The only real difference between [`SharedArc::borrow`] and [`SharedArc::update_mut`] is
    /// that this method notifies other uses of an update and returns a type supporting
    /// [`DerefMut`].
    pub fn update_mut(&self, mgr: &mut EventMgr) -> SharedArcRefMut<T> {
        mgr.update_with_id((self.0).0, 0);
        let mut inner = (self.0).1.lock().unwrap();
        inner.1 += 1;
        SharedArcRefMut(inner)
    }
}

impl<T: Clone + Debug + 'static> SharedData for SharedArc<T> {
    type Key = ();
    type Item = T;
    type ItemRef<'b> = SharedArcRef<'b, T>;

    fn version(&self) -> u64 {
        (self.0).1.lock().unwrap().1
    }

    fn contains_key(&self, _: &()) -> bool {
        true
    }
    fn borrow(&self, _: &Self::Key) -> Option<Self::ItemRef<'_>> {
        Some(SharedArcRef((self.0).1.lock().unwrap()))
    }

    fn update(&self, mgr: &mut EventMgr, _: &(), item: Self::Item) {
        let mut inner = (self.0).1.lock().unwrap();
        inner.0 = item;
        inner.1 += 1;
        mgr.update_with_id((self.0).0, 0);
    }
}
