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
    // TODO: note that we do not return a reference for compatibility with Rc, RefCell, Mutex etc.
    // Investigate using a guard/lock for data access?
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

// TODO(spec): implement Accessor<I> and AccessorShared<I> for T: Deref + 'static
// where <T as Deref>::Target: Accessor<I>
// Instead we implement for a few more specific types

impl<I, T: Accessor<I> + ?Sized> Accessor<I> for &'static T {
    type Item = T::Item;
    fn len(&self) -> I {
        (**self).len()
    }
    fn get(&self, index: I) -> Self::Item {
        (**self).get(index)
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        (**self).update_handle()
    }
}
impl<I, T: AccessorShared<I> + ?Sized> AccessorShared<I> for &'static T {
    fn set(&self, index: I, value: Self::Item) -> UpdateHandle {
        (**self).set(index, value)
    }
}

impl<I, T: Accessor<I> + ?Sized> Accessor<I> for Rc<T> {
    type Item = T::Item;
    fn len(&self) -> I {
        (**self).len()
    }
    fn get(&self, index: I) -> Self::Item {
        (**self).get(index)
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        (**self).update_handle()
    }
}
impl<I, T: AccessorShared<I> + ?Sized> AccessorShared<I> for Rc<T> {
    fn set(&self, index: I, value: Self::Item) -> UpdateHandle {
        (**self).set(index, value)
    }
}

impl<I, T: Accessor<I> + ?Sized> Accessor<I> for RefCell<T> {
    type Item = T::Item;
    fn len(&self) -> I {
        self.borrow().len()
    }
    fn get(&self, index: I) -> Self::Item {
        self.borrow().get(index)
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        self.borrow().update_handle()
    }
}

impl<I, T: AccessorShared<I> + ?Sized> AccessorShared<I> for RefCell<T> {
    fn set(&self, index: I, value: Self::Item) -> UpdateHandle {
        self.borrow_mut().set(index, value)
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

/// Types "probably" representable by `usize`
///
/// We use range checks in debug mode
pub trait ToFromUsize: Debug + Sized + 'static {
    fn from_usize(i: usize) -> Self;
    fn to_usize(i: Self) -> usize;
    fn from_u32(i: u32) -> Self {
        Self::from_usize(kas::text::conv::to_usize(i))
    }
    fn to_u32(i: Self) -> u32 {
        kas::text::conv::to_u32(Self::to_usize(i))
    }
}

impl ToFromUsize for usize {
    fn from_usize(i: usize) -> Self {
        i
    }
    fn to_usize(i: Self) -> usize {
        i
    }
}
impl ToFromUsize for u32 {
    fn from_usize(i: usize) -> Self {
        kas::text::conv::to_u32(i)
    }
    fn to_usize(i: Self) -> usize {
        kas::text::conv::to_usize(i)
    }
    fn from_u32(i: u32) -> Self {
        i
    }
    fn to_u32(i: Self) -> u32 {
        i
    }
}

/// Filter accessor over another accessor
#[derive(Clone, Debug)]
pub struct FilterAccessor<I: ToFromUsize, T: Accessor<I>> {
    data: T,
    view: Vec<u32>,
    update: UpdateHandle,
    _i: std::marker::PhantomData<I>,
}

impl<I: ToFromUsize, T: Accessor<I>> FilterAccessor<I, T> {
    /// Construct, with all data hidden (filtered out)
    ///
    /// This is the fastest constructor.
    pub fn new_hidden(data: T) -> Self {
        let view = Vec::with_capacity(I::to_usize(data.len()));
        FilterAccessor {
            data,
            view,
            update: UpdateHandle::new(),
            _i: Default::default(),
        }
    }

    /// Construct, with all data visible
    pub fn new_visible(data: T) -> Self {
        let mut x = Self::new_hidden(data);
        x.view.extend(0..I::to_u32(x.data.len()));
        x
    }

    /// Update the filtered view
    ///
    /// An update should be triggered using the returned handle.
    pub fn update_filter<F: Fn(T::Item) -> bool>(&mut self, filter: F) -> UpdateHandle {
        self.view.clear();
        // TODO: is this slow?
        for i in 0..I::to_u32(self.data.len()) {
            if filter(self.data.get(I::from_u32(i))) {
                self.view.push(i);
            }
        }
        self.update
    }
}

impl<I: ToFromUsize, T: Accessor<I>> Accessor<I> for FilterAccessor<I, T> {
    type Item = T::Item;
    fn len(&self) -> I {
        I::from_usize(self.view.len())
    }
    fn get(&self, index: I) -> Self::Item {
        self.data.get(I::from_u32(self.view[I::to_usize(index)]))
    }
    fn update_handle(&self) -> Option<UpdateHandle> {
        Some(self.update)
    }
}
