// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Class-specific widget traits

use crate::{CowString, TkAction};

/// Functionality for widgets which can be toggled or selected: check boxes,
/// radio buttons, toggle switches.
///
/// The value `true` means *checked*, *selected* or *toggled on*.
pub trait HasBool {
    /// Get the widget's state
    fn get_bool(&self) -> bool;

    /// Set the widget's state
    fn set_bool(&mut self, state: bool) -> TkAction;
}

/// Functionality for widgets with visible text.
///
/// This applies to both labels and the text content of interactive widgets.
/// The only widgets supporting both labels and interactive content have
/// boolean values (e.g. checkboxes); these may support *both* `HasText` and
/// [`HasBool`].
pub trait HasText {
    /// Get the widget's text.
    fn get_text(&self) -> &str;

    /// Set the widget's text.
    fn set_text<T: Into<CowString>>(&mut self, text: T) -> TkAction
    where
        Self: Sized,
    {
        self.set_cow_string(text.into())
    }

    /// Set the widget's text ([`CowString`])
    ///
    /// This method is for implementation. It is recommended to use
    /// [`HasText::set_text`] instead.
    fn set_cow_string(&mut self, text: CowString) -> TkAction;
}

/// Additional functionality required by the `EditBox` widget.
pub trait Editable: HasText {
    /// Get whether this input field is editable.
    fn is_editable(&self) -> bool;

    /// Set whether this input field is editable.
    fn set_editable(&mut self, editable: bool);
}

/// Summation of [`HasBool`] and [`HasText`] traits.
///
/// Used because Rust doesn't (yet) support multi-trait objects.
pub trait HasBoolText: HasBool + HasText {}

impl<T> HasBoolText for T where T: HasBool + HasText {}
