// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Class-specific widget traits
//!
//! These traits provide generic ways to interact with common widget properties,
//! e.g. to read the text of a `Label` or set the state of a `CheckBox`.

use crate::text::{AccelString, FormattedString};
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
///
/// Rich-text APIs may also support `HasString`; in this case reading returns
/// the text without formatting information and writing discards any existing
/// formatting.
pub trait HasString {
    /// Get text by reference
    fn get_str(&self) -> &str;

    /// Get text as a `String`
    #[inline]
    fn get_string(&self) -> String {
        self.get_str().to_string()
    }

    /// Set text from a `&str`
    #[inline]
    fn set_str(&mut self, text: &str) -> TkAction {
        self.set_string(text.to_string())
    }

    /// Set text from a string
    fn set_string(&mut self, text: String) -> TkAction;
}

/// Read / write a formatted `String`
pub trait HasFormatted {
    /// Get text as a `String`
    fn get_formatted(&self) -> FormattedString;

    /// Set from a formatted string
    fn set_formatted<S: Into<FormattedString>>(&mut self, text: S) -> TkAction
    where
        Self: Sized,
    {
        self.set_formatted_string(text.into())
    }

    /// Set from a formatted string
    fn set_formatted_string(&mut self, text: FormattedString) -> TkAction;
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
