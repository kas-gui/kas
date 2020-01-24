// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Class-specific widget traits

use crate::event::Manager;

/// Functionality for widgets which can be toggled or selected: check boxes,
/// radio buttons, toggle switches.
///
/// The value `true` means *checked*, *selected* or *toggled on*.
pub trait HasBool {
    /// Get the widget's state
    fn get_bool(&self) -> bool;

    /// Set the widget's state
    fn set_bool(&mut self, mgr: &mut Manager, state: bool);
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
    fn set_text<T: ToString>(&mut self, mgr: &mut Manager, text: T)
    where
        Self: Sized,
    {
        self.set_string(mgr, text.to_string());
    }

    /// Set the widget's text (string only).
    ///
    /// This method is for implementation.
    fn set_string(&mut self, mgr: &mut Manager, text: String);
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
