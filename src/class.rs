// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Class-specific widget traits
//!
//! These traits provide generic ways to interact with common widget properties,
//! e.g. to read the text of a `Label` or set the state of a `CheckBox`.

use crate::TkAction;

/// Read / write a boolean value
///
/// The value `true` means *checked*, *selected* or *toggled on*.
pub trait HasBool {
    /// Get the widget's state
    fn get_bool(&self) -> bool;

    /// Set the widget's state
    fn set_bool(&mut self, state: bool) -> TkAction;
}

/// Write a plain-text value or label
pub trait SetText {
    /// Set text (unformatted)
    fn set_text<T: ToString>(&mut self, text: T) -> TkAction
    where
        Self: Sized,
    {
        self.set_string(text.to_string())
    }

    /// Set text from a `String`
    ///
    /// Depending on the widget, this may set a label or a value.
    fn set_string(&mut self, text: String) -> TkAction;
}

/// Read a plain-text value / label
///
/// This is an extension over [`SetText`] allowing text to be read.
///
/// Note that widgets may support setting a plain-text label or value without
/// supporting reading a plain text value, for example since rich-text labels
/// are not easily converted to a plain-text representation.
pub trait HasText: SetText {
    /// Get the widget's text value (as plain text)
    fn get_text(&self) -> &str;
}

/// Read a rich text value / label
pub trait HasRichText {
    // TODO: set_rich_text and auto impls?
    /// Get the widget's text label as rich text
    fn clone_rich_text(&self) -> kas::text::RichText;
}
