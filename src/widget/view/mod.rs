// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

use super::Label;
#[allow(unused)]
use kas::event::Manager;
use kas::event::UpdateHandle;
use kas::prelude::*;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

// mod list;
mod single;

// pub use list::ListView;
pub use single::SingleView;

/// View widgets
///
/// Implementors are able to view data of type `T`.
/// Note: we pass `&T` to better match up with [`Accessor::get`].
pub trait ViewWidget<T>: Widget {
    /// Construct a default instance (with no data)
    fn default() -> Self
    where
        T: Default;
    /// Construct an instance from a data value
    fn new(data: T) -> Self;
    /// Set the viewed data
    fn set(&mut self, data: T) -> TkAction;
}

//TODO(spec): enable this as a specialisation of the T: ToString impl
// In the mean-time we only lose the Markdown impl by disabling this
/*
impl<T: Clone + Default + FormattableText + 'static> ViewWidget<T> for Label<T> {
    fn default() -> Self {
        Label::new(T::default())
    }
    fn new(data: T) -> Self {
        Label::new(data.clone())
    }
    fn set(&mut self, data: &T) -> TkAction {
        self.set_text(data.clone())
    }
}
*/

impl<T: Default + ToString> ViewWidget<T> for Label<String> {
    fn default() -> Self {
        Label::new(T::default().to_string())
    }
    fn new(data: T) -> Self {
        Label::new(data.to_string())
    }
    fn set(&mut self, data: T) -> TkAction {
        self.set_text(data.to_string())
    }
}

/// Default view assignments
///
/// This trait may be implemented to assign a default view widget to a specific
/// data type.
pub trait DefaultView: Sized {
    type Widget: ViewWidget<Self>;
}

// TODO(spec): enable this over more specific implementations
/*
impl<T: Clone + Default + FormattableText + 'static> DefaultView for T {
    type Widget = Label<T>;
}
*/
impl DefaultView for String {
    type Widget = Label<String>;
}
impl<'a> DefaultView for &'a str {
    type Widget = Label<String>;
}

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

/// Wrapper for shared constant data
///
/// This may be useful with static data, e.g. `[&'static str]`.
#[derive(Clone, Debug, Default)]
pub struct SharedConst<T: Debug + 'static + ?Sized>(T);

impl<T: Clone + Debug + 'static + ?Sized> SharedConst<T> {
    /// Construct with given data
    pub fn new(data: T) -> Self {
        SharedConst(data)
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

impl<T: Clone + Debug + 'static + ?Sized> Accessor<usize> for SharedConst<[T]> {
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
