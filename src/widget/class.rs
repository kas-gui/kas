// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget classifications
//!
//! Each widget must have one of the classifications defined in the [`Class`]
//! enumeration. In some of these cases, the widget must implement additional
//! functionality (usually on itself).

use crate::traits::{Editable, HasBoolText, HasText};
use std::fmt;

/// Widget classifications
pub enum Class<'a> {
    Container,
    // Dialog,
    Label(&'a dyn HasText),
    Entry(&'a dyn Editable),
    Button(&'a dyn HasText),
    CheckBox(&'a dyn HasBoolText),
    Frame,
    Window,
}

impl<'a> fmt::Debug for Class<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Class::{}",
            match self {
                Class::Container => "Container",
                Class::Label(_) => "Label",
                Class::Entry(_) => "Entry",
                Class::Button(_) => "Button",
                Class::CheckBox(_) => "CheckBox",
                Class::Frame => "Frame",
                Class::Window => "Window",
            }
        )
    }
}
