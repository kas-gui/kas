//! Canvas types

use super::{Widget, WidgetCore};
use super::event;

pub struct Text {
    text: &'static str,
}

impl Text {
    pub fn set_text(&mut self, s: String) {
        unimplemented!()
    }
}

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Self {
        Text { text }
    }
}

impl WidgetCore for Text {}

impl Widget for Text {
    type Response = event::NoResponse;
}
