// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! View widgets
//!
//! View widgets exist as a view over some shared data.

use kas::prelude::*;
use kas::widget::Label;

/// View widgets
///
/// Implementors are able to view data of type `T`.
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
