//! Canvas types

use super::Widget;
use super::event;

pub struct Text {
    text: &'static str,
}

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Self {
        Text { text }
    }
}

impl Widget for Text {
    type Response = event::NoResponse;
}
