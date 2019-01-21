// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget functionality

use crate::TkWidget;


/// Functionality for widgets with visible text
pub trait HasText {
    /// Get the widget's text.
    fn get_text(&self) -> &str;
    
    /// Set the widget's text.
    fn set_text(&mut self, tk: &TkWidget, text: &str);
}

/// Additional functionality required by the `Entry` class.
pub trait Editable: HasText {
    /// Get whether this input field is editable.
    fn is_editable(&self) -> bool;
}
