//! Display widgets show information but are not interactive

use crate::event;
use crate::macros::Widget;
use crate::toolkit::Toolkit;
use crate::widget::{Class, Core, CoreData};

#[widget(class = Class::Text, label = Some(self.text.as_str()))]
#[derive(Clone, Default, Debug, Widget)]
pub struct Text {
    #[core] core: CoreData,
    text: String,
}

impl_layout_simple!(Text);

impl Text {
    pub fn set_text(&mut self, tk: &Toolkit, text: &str) {
        self.text = String::from(text);
        tk.tk_widget().set_label(self.tkd(), text);
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
