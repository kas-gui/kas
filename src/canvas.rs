//! Canvas types

use ::Widget;

pub struct Text {
    text: &'static str,
}

impl From<&'static str> for Text {
    fn from(text: &'static str) -> Text {
        Text { text }
    }
}

impl Widget for Text {
}
