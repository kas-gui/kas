// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Display widgets show information but are not interactive

use crate::macros::Widget;
use crate::event::{Handler, NoResponse};
use crate::{Class, Core, CoreData, TkWidget, Widget};

#[layout]
#[widget(class = Class::Text, label = Some(self.text.as_str()))]
#[derive(Clone, Default, Debug, Widget)]
pub struct Text {
    #[core] core: CoreData,
    text: String,
}

impl Text {
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
}

impl Widget for Entry {
    fn class(&self) -> Class { Class::Entry }
    
    fn label(&self) -> Option<&str> { Some(self.text.as_str()) }
    
    fn is_editable(&self) -> bool { self.editable }
    
    fn len(&self) -> usize { 0 }
    
    fn get(&self, _: usize) -> Option<&Widget> { None }
    
    fn get_mut(&mut self, _: usize) -> Option<&mut Widget> { None }
}

impl Handler for Entry {
    type Response = NoResponse;
}
