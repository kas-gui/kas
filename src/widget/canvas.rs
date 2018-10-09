//! Canvas types

use crate::event;
use crate::widget::{Class, Widget, CoreData, WidgetCore};
use crate::toolkit::Toolkit;

#[derive(Clone, Default, Debug)]
pub struct Text {
    core: CoreData,
    text: String,
}

impl_widget_core!(Text; core);
impl_layout_simple!(Text);

impl Widget for Text {
    fn class(&self) -> Class { Class::Text }
    fn label(&self) -> Option<&str> { Some(self.text.as_str()) }
    
    fn len(&self) -> usize { 0 }
    fn get(&self, _index: usize) -> Option<&Widget> { None }
    fn get_mut(&mut self, _index: usize) -> Option<&mut Widget> { None }
}

impl Text {
    pub fn set_text(&mut self, tk: &Toolkit, text: &str) {
        self.text = String::from(text);
        tk.tk_widget().set_label(self.get_tkd(), text);
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

impl event::Handler for Text {
    type Response = event::NoResponse;
}
