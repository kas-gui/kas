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
/// version counter.
///
/// The wrapped value may be read via [`Self::borrow`], [`Self::try_borrow`] and
/// [`SharedData::get_cloned`].
///
/// The value may be set via [`SharedData::update`].
///
/// This wrapper type may be useful for simple shared data, but for more complex
/// uses a custom wrapper type may be required.
#[derive(Clone, Debug)]
pub struct SharedRc<T: Debug>(Rc<(UpdateId, RefCell<(T, u64)>)>);

impl<T: Debug + Default> Default for SharedRc<T> {
    fn default() -> Self {
        SharedRc(Rc::new((UpdateId::new(), Default::default())))
    }
}

/// A mutably borrowed reference
pub struct SharedRcRefMut<'a, T>(RefMut<'a, (T, u64)>);
impl<'a, T> Deref for SharedRcRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.deref().0
    }
}
impl<'a, T> DerefMut for SharedRcRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0.deref_mut().0
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

    /// Mutably borrows the wrapped value, notifying other users of an update.
    pub fn update_mut(&self, mgr: &mut EventMgr) -> SharedRcRefMut<T> {
        mgr.update_with_id((self.0).0, 0);
        let mut cell = (self.0).1.borrow_mut();
        cell.1 += 1;
        SharedRcRefMut(cell)
    }
}

impl<T: Clone + Debug + 'static> SharedData for SharedRc<T> {
    type Key = ();
    type Item = T;
    type ItemRef<'b> = Ref<'b, T>;

    fn version(&self) -> u64 {
        (self.0).1.borrow().1
    }

    fn contains_key(&self, _: &()) -> bool {
        true
    }
    fn borrow(&self, _: &Self::Key) -> Option<Self::ItemRef<'_>> {
        Some(Ref::map((self.0).1.borrow(), |tuple| &tuple.0))
    }

    fn update(&self, mgr: &mut EventMgr, _: &(), item: Self::Item) {
        let mut cell = (self.0).1.borrow_mut();
        cell.0 = item;
        cell.1 += 1;
        mgr.update_with_id((self.0).0, 0);
    }
}
