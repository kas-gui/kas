// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Display widgets show information but are not interactive

use crate::macros::Widget;
use crate::event::NoResponse;
use crate::{Class, Core, CoreData, HasText, Editable, TkWidget};

/// A simple text display widget
#[widget(class = Class::Text(self))]
#[handler(response = NoResponse)]
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
}

impl<T> From<T> for Text where String: From<T> {
    fn from(text: T) -> Self {
        Text {
            core: Default::default(),
            text: String::from(text)
        }
    }
}

impl HasText for Text {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
        self.text = text.into();
    }
}


/// Basic text entry.
#[widget(class = Class::Entry(self))]
#[handler(response = NoResponse)]
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
}

impl Entry {
    pub fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
    }
}

impl HasText for Entry {
    fn get_text(&self) -> &str {
        &self.text
    }
    
    fn set_text(&mut self, tk: &TkWidget, text: &str) {
        tk.set_label(self.tkd(), text);
        self.text = text.into();
    }
}

impl Editable for Entry {
    fn is_editable(&self) -> bool {
        self.editable
    }
}
