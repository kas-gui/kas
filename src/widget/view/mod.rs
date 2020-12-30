// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

// TODO: how do we notify widgets holding an Accessor when an update is required?
// TODO: how do we allow fine-grained updates when a subset of data changes?

use super::Label;
use kas::event::UpdateHandle;
use kas::prelude::*;
use kas::text::format::FormattableText;
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
    fn default() -> Self;
    /// Construct an instance from a data value
    fn new(data: T) -> Self;
    /// Set the viewed data
    fn set(&mut self, data: &T) -> TkAction;
}

impl<T: Clone + Default + FormattableText + 'static> ViewWidget<T> for Label<T> {
    fn default() -> Self {
        Default::default()
    }
    fn new(data: T) -> Self {
        Self::new(data.clone())
    }
    fn set(&mut self, data: &T) -> TkAction {
        self.set_text(data.clone())
    }
}

/// Default view assignments
///
/// This trait may be implemented to assign a default view widget to a specific
/// data type.
pub trait DefaultView: Sized {
    type Widget: ViewWidget<Self>;
}

impl<T: Clone + Default + FormattableText + 'static> DefaultView for T {
    type Widget = Label<T>;
}

/// Base trait required by view widgets
// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
// Note: since there can be at most one impl for any (T, Self), it would make
// sense for I to be an associated type; BUT this would make our generic impls
// conflict (e.g. downstream *could* write `impl AsRef<S> for [S] { .. }`).
pub trait Accessor<I, T: ?Sized>: Debug + 'static {
    /// Size descriptor
    ///
    /// Note: for `I == ()` we consider `()` a valid index; in other cases we
    /// usually expect `index < accessor.len()` (for each component).
    fn len(&self) -> I;

    /// Access data by index
    fn get(&self, index: I) -> T;

    /// Get an update handle, if any is used
    ///
    /// Widgets may use this `handle` to call `mgr.update_on_handle(handle, self.id())`.
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}

/// Extension trait for shared data for view widgets
pub trait AccessorShared<I, T: ?Sized>: Accessor<I, T> {
    /// Set data at the given index
    ///
    /// The caller is expected to arrange synchronisation as necessary, likely
    /// using [`Accessor::update_handle`].
    fn set(&mut self, index: I, value: T);
}

impl<T: Clone + Debug + 'static> Accessor<usize, T> for [T] {
    fn len(&self) -> usize {
        self.len()
    }
    fn get(&self, index: usize) -> T {
        self[index].clone()
    }
}

impl<T: Clone + Debug + 'static> Accessor<(), T> for T {
    fn len(&self) -> () {
        ()
    }
    fn get(&self, _: ()) -> T {
        self.clone()
    }
}

/// Wrapper for single-thread shared data
#[derive(Clone, Debug)]
pub struct SharedRc<T: Clone + Debug + 'static> {
    handle: UpdateHandle,
    data: Rc<RefCell<T>>,
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

impl<T: Clone + Debug + 'static> Accessor<(), T> for SharedRc<T> {
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

impl<T: Clone + Debug + 'static> AccessorShared<(), T> for SharedRc<T> {
    fn set(&mut self, _: (), value: T) {
        *self.data.borrow_mut() = value;
    }
}
