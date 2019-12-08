// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget classifications
//!
//! Each widget must have one of the classifications defined in the [`Class`]
//! enumeration. In some of these cases, the widget must implement additional
//! functionality (usually on itself).

use crate::TkWindow;
use std::fmt;

/// Alignment of contents
pub enum Align {
    /// Align to top or left (for left-to-right text)
    Begin,
    /// Align to centre
    Center,
    /// Align to bottom or right (for left-to-right text)
    End,
    /// Attempt to align to both margins, padding with space
    Justify,
}

/// Widget classifications
pub enum Class {
    None, // temporary
    Container,
    // Dialog,
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Class::{}",
            match self {
                Class::None => "None",
                Class::Container => "Container",
            }
        )
    }
}

impl Class {
    /// Does this widget allow keyboard focus?
    pub fn allow_focus(&self) -> bool {
        match self {
            Class::None | Class::Container => false,
        }
    }
}

/// Functionality for widgets which can be toggled or selected: check boxes,
/// radio buttons, toggle switches.
///
/// The value `true` means *checked*, *selected* or *toggled on*.
pub trait HasBool {
    /// Get the widget's state
    fn get_bool(&self) -> bool;

    /// Set the widget's state
    fn set_bool(&mut self, tk: &mut dyn TkWindow, state: bool);
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
    fn set_text<T: ToString>(&mut self, tk: &mut dyn TkWindow, text: T)
    where
        Self: Sized,
    {
        self.set_string(tk, text.to_string());
    }

    /// Set the widget's text (string only).
    ///
    /// This method is for implementation.
    fn set_string(&mut self, tk: &mut dyn TkWindow, text: String);
}

/// Additional functionality required by the `Entry` class.
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
