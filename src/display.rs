// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Display widgets show information but are not interactive

use crate::macros::Widget;
use crate::event::{Handler, NoResponse};
use crate::{Class, Core, CoreData, TkWidget};

/// A simple text display widget
#[layout]
#[widget(class = Class::Text, label = Some(self.text.as_str()))]
#[derive(Clone, Default, Debug, Widget)]
pub struct Text {
    #[core] core: CoreData,
    text: String,
}

impl Text {
    /// Construct a new, empty instance
    pub fn new() -> Self {
        Text {
            core: Default::default(),
            text: String::new()
        }
    }
    
    /// Set the text to display
    pub fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
    }
}

impl<T> From<T> for Text where String: From<T> {
    fn from(text: T) -> Self {
        Text {
            core: Default::default(),
            text: String::from(text)
        }
    }
}

impl Handler for Text {
    type Response = NoResponse;
}


/// Basic text entry.
/// 
/// TODO: this is currently just a hack to satisfy a single use-case.
#[layout]
#[widget(class = Class::Entry, label = Some(self.text.as_str()))]
#[derive(Clone, Default, Debug, Widget)]
pub struct Entry {
    #[core] core: CoreData,
    editable: bool,
    text: String,
}

impl Entry {
    pub fn new(editable: bool, text: String) -> Self {
        Entry {
            core: Default::default(),
            editable,
            text,
        }
    }
    
    pub fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
    }
    
    /// Get whether this input field is editable.
    pub fn is_editable(&self) -> bool {
        self.editable
    }
}

impl Handler for Entry {
    type Response = NoResponse;
}
