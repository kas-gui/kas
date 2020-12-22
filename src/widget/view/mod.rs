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
use kas::prelude::*;
use kas::text::format::FormattableText;
use std::fmt::Debug;

mod list;

pub use list::ListView;

/// View widgets
///
/// Implementors are able to view data of type `T`.
pub trait ViewWidget<T>: Widget {
    /// Construct a default instance (with no data)
    fn default() -> Self;
    /// Construct an instance from a data value
    fn new(data: T) -> Self;
    /// Set the viewed data
    fn set(&mut self, data: T) -> TkAction;
}

impl<T: Default + FormattableText + 'static> ViewWidget<T> for Label<T> {
    fn default() -> Self {
        Default::default()
    }
    fn new(data: T) -> Self {
        Self::new(data)
    }
    fn set(&mut self, data: T) -> TkAction {
        self.set_text(data)
    }
}

/// Default view assignments
///
/// This trait may be implemented to assign a default view widget to a specific
/// data type.
pub trait DefaultView: Sized {
    type Widget: ViewWidget<Self>;
}

impl DefaultView for &'static str {
    type Widget = Label<&'static str>;
}
impl DefaultView for String {
    type Widget = Label<String>;
}

// Note: we require Debug + 'static to allow widgets using this to implement
// WidgetCore, which requires Debug + Any.
pub trait Accessor<Index: Copy>: Debug + 'static {
    type Item;
    fn len(&self) -> Index;
    fn get(&self, index: Index) -> Self::Item;
}

impl<T: Clone + Debug + 'static> Accessor<usize> for [T] {
    type Item = T;
    fn len(&self) -> usize {
        self.len()
    }
    fn get(&self, index: usize) -> T {
        self[index].clone()
    }
}

impl<Index: Copy, A: Accessor<Index> + ?Sized> Accessor<Index> for &'static A {
    type Item = A::Item;
    fn len(&self) -> Index {
        (*self).len()
    }
    fn get(&self, index: Index) -> Self::Item {
        (*self).get(index)
    }
}
