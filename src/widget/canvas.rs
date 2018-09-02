//! Canvas types

use super::{Widget, WidgetCoreData};
use super::event;
use super::layout::WidgetLayout;

#[derive(Clone, Default)]
pub struct Text {
    core: WidgetCoreData,
    text: String,
}

impl_widget_core!(Text, core);

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

impl WidgetLayout for Text {
    fn min_size(&self) -> (i32, i32) {
        (80, 40)    // TODO
    }
}

impl Widget for Text {
    type Response = event::NoResponse;
}
