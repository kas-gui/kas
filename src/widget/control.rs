//! Basic controls

use super::Widget;

// TODO: abstract out text part?
pub struct TextButton {
    msg: &'static str,
}

impl TextButton {
    pub fn new(msg: &'static str) -> Self {
        TextButton { msg }
    }
}

impl Widget for TextButton {}

pub mod button {
    use super::TextButton;
    
    pub fn ok() -> TextButton {
        TextButton::new("Ok")
    }
}
