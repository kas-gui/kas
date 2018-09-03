//! Canvas types

use event;
use widget::{Class, Layout, Widget, CoreData};

#[derive(Clone, Default)]
pub struct Text {
    core: CoreData,
    text: String,
}

impl_widget_core!(Text, core);

impl Widget for Text {
    fn class(&self) -> Class { Class::Text }
    fn label(&self) -> Option<&str> { Some(self.text.as_str()) }
    
    fn len(&self) -> usize { 0 }
    fn get(&self, index: usize) -> Option<&(dyn Widget + 'static)> { None }
}

impl Text {
    pub fn set_text<T>(&mut self, s: T) where String: From<T> {
        self.text = String::from(s);
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

impl Layout for Text {
    fn min_size(&self) -> (i32, i32) {
        (80, 40)    // TODO
    }
}

impl event::Handler for Text {
    type Response = event::NoResponse;
}
