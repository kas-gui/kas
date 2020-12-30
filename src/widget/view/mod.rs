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
use std::fmt::Debug;

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
    fn new(data: &T) -> Self;
    /// Set the viewed data
    fn set(&mut self, data: &T) -> TkAction;
}

impl<T: Clone + Default + FormattableText + 'static> ViewWidget<T> for Label<T> {
    fn default() -> Self {
        Default::default()
    }
    fn new(data: &T) -> Self {
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

// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
// Note: since there can be at most one impl for any (T, Self), it would make
// sense for I to be an associated type; BUT this would make our generic impls
// conflict (e.g. downstream *could* write `impl AsRef<S> for [S] { .. }`).
pub trait Accessor<I, T: ?Sized>: Debug + 'static {
    fn len(&self) -> I;
    fn get(&self, index: I) -> &T;
    fn update_handle(&self) -> Option<UpdateHandle> {
        None
    }
}

pub trait AccessorMut<I, T: ?Sized>: Accessor<I, T> {
    fn set(&mut self, index: I, value: T);
}

impl<T: Debug + 'static> Accessor<usize, T> for [T] {
    fn len(&self) -> usize {
        self.len()
    }
    fn get(&self, index: usize) -> &T {
        &self[index]
    }
}

impl<T: ?Sized, R: AsRef<T> + Debug + ?Sized + 'static> Accessor<(), T> for R {
    fn len(&self) -> () {
        ()
    }
    fn get(&self, _: ()) -> &T {
        self.as_ref()
    }
}
