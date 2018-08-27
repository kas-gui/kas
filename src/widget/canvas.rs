//! Canvas types

use super::{Widget, WidgetCore};
use super::event;
use super::layout::WidgetLayout;

pub struct Text {
    text: &'static str,
    size: (u32, u32),
}

impl Text {
    pub fn set_text(&mut self, s: String) {
        unimplemented!()
    }
}

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Self {
        Text { text, size: (0, 0) }
    }
}

impl WidgetLayout for Text {
    fn min_size(&self) -> (u32, u32) {
        (80, 40)    // TODO
    }

    fn set_size(&mut self, size: (u32, u32)) {
        self.size = size;
    }
}

impl WidgetCore for Text {}

impl Widget for Text {
    type Response = event::NoResponse;
}
