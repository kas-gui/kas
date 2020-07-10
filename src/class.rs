// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Class-specific widget traits
//!
//! These traits provide generic ways to interact with common widget properties,
//! e.g. to read the text of a `Label` or set the state of a `CheckBox`.

use crate::string::AccelString;
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

/// Read / write an unformatted `String`
pub trait HasString {
    /// Get text by reference
    fn get_str(&self) -> &str;

    /// Get text as a `String`
    fn get_string(&self) -> String {
        self.get_str().to_string()
    }

    /// Set text from an unformatted string
    fn set_string(&mut self, text: String) -> TkAction;
}

/// Read a (rich) text value
pub trait CloneText {
    /// Clone text as a plain `String`
    ///
    /// For rich-text representations this strips formatting.
    ///
    /// An implementation is provided based on [`CloneText::clone_text`],
    /// though where an unformatted representation is available internally this
    /// may be used for a more efficient implementation.
    fn clone_string(&self) -> String {
        self.clone_text().to_string()
    }

    /// Clone text as rich text
    ///
    /// Can be implemented via `self.clone_string().into()` if there is no
    /// rich-text representation.
    fn clone_text(&self) -> kas::text::RichText;
}

// TODO(spec): it would be nice to provide this implementation
// impl<T: HasString> CloneText for T {
//     fn clone_string(&self) -> String {
//         self.get_string().to_string()
//     }
//
//     fn clone_text(&self) -> kas::text::RichText {
//         self.get_string().into()
//     }
// }

/// Set a text value
///
/// TODO: add convenience methods for parsing, e.g. `set_html`.
pub trait SetText: CloneText {
    /// Set text
    ///
    /// This method supports [`kas::text::RichText`] for formatted input and
    /// `String` and `&str` for unformatted input.
    fn set_text<T: Into<kas::text::RichText>>(&mut self, text: T) -> TkAction
    where
        Self: Sized,
    {
        self.set_rich_text(text.into())
    }

    /// Set rich text
    fn set_rich_text(&mut self, text: kas::text::RichText) -> TkAction;
}

/// Set a control label
///
/// Control labels do not support rich-text formatting but do support
/// accelerator keys, identified via a `&` prefix (e.g. `&File`).
pub trait SetAccel {
    /// Set text
    ///
    /// This method supports [`AccelString`], `String` and `&str` as input.
    /// The latter are parsed for accel keys identified by `&` prefix.
    fn set_accel<T: Into<AccelString>>(&mut self, accel: T) -> TkAction
    where
        Self: Sized,
    {
        self.set_accel_string(accel.into())
    }

    /// Set accel string
    fn set_accel_string(&mut self, accel: AccelString) -> TkAction;
}
