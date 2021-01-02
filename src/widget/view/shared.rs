// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shared data for view widgets

#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

/// Base trait required by view widgets
// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
pub trait Accessor<I>: Debug + 'static {
    type Item;

    /// Size descriptor
    ///
    /// Note: for `I == ()` we consider `()` a valid index; in other cases we
    /// usually expect `index < accessor.len()` (for each component).
    fn len(&self) -> I;

    /// Access data by index
    fn get(&self, index: I) -> Self::Item;

    /// Get an update handle, if any is used
    ///
    /// Widgets may use this `handle` to call `mgr.update_on_handle(handle, self.id())`.
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}

/// Extension trait for shared data for view widgets
pub trait AccessorShared<I>: Accessor<I> {
    /// Set data at the given index
    ///
    /// The caller should call [`Manager::trigger_update`] using the returned
    /// update handle, using an appropriate transformation of the index for the
    /// payload (the transformation defined by implementing view widgets).
    /// Calling `trigger_update` is unnecessary before the UI has been started.
    fn set(&self, index: I, value: Self::Item) -> UpdateHandle;
}

impl<I, T: Debug + Deref + 'static> Accessor<I> for T
where
    <T as Deref>::Target: Accessor<I>,
{
    type Item = <<T as Deref>::Target as Accessor<I>>::Item;
    fn len(&self) -> I {
        self.deref().len()
    }
    fn get(&self, index: I) -> Self::Item {
        self.deref().get(index)
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        self.deref().update_handle()
    }
}

impl<I, T: Debug + Deref + 'static> AccessorShared<I> for T
where
    <T as Deref>::Target: AccessorShared<I>,
{
    fn set(&self, index: I, value: Self::Item) -> UpdateHandle {
        self.deref().set(index, value)
    }
}

/// Wrapper for shared constant data
///
/// This may be useful with static data, e.g. `[&'static str]`.
#[derive(Clone, Debug, Default)]
pub struct SharedConst<T: Debug + 'static + ?Sized>(T);

impl<T: Debug + 'static> SharedConst<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedConst(data)
    }
}

impl<T: Debug + 'static> From<T> for SharedConst<T> {
    fn from(data: T) -> Self {
        SharedConst(data)
    }
}

impl<T: Debug + 'static + ?Sized> From<&T> for &SharedConst<T> {
    fn from(data: &T) -> Self {
        // SAFETY: SharedConst<T> is a thin wrapper around T
        unsafe { &*(data as *const T as *const SharedConst<T>) }
    }
}

impl<T: Clone + Debug + 'static> Accessor<()> for SharedConst<T> {
    type Item = T;
    fn len(&self) -> () {
        ()
    }
    fn get(&self, _: ()) -> T {
        self.0.clone()
    }
}

impl<T: Clone + Debug + 'static> Accessor<usize> for SharedConst<[T]> {
    type Item = T;
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> T {
        self.0[index].to_owned()
    }
}

/// Wrapper for single-thread shared data
#[derive(Clone, Debug)]
pub struct SharedRc<T: Clone + Debug + 'static> {
    handle: UpdateHandle,
    data: Rc<RefCell<T>>,
}

impl<T: Default + Clone + Debug + 'static> Default for SharedRc<T> {
    fn default() -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Default::default(),
        }
    }
}

impl<T: Clone + Debug + 'static> SharedRc<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedRc {
            handle: UpdateHandle::new(),
            data: Rc::new(RefCell::new(data)),
        }
    }
}

impl<T: Clone + Debug + 'static> Accessor<()> for SharedRc<T> {
    type Item = T;
    fn len(&self) -> () {
        ()
    }
    fn get(&self, _: ()) -> T {
        self.data.borrow().to_owned()
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.handle)
    }
}

impl<T: Clone + Debug + 'static> AccessorShared<()> for SharedRc<T> {
    fn set(&self, _: (), value: T) -> UpdateHandle {
        *self.data.borrow_mut() = value;
        self.handle
    }
}
