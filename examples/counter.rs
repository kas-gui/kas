//! Counter example (simple button)

extern crate mygui;

use mygui::widget::{
    canvas::Text,
    control::TextButton,
    event::NoResponse,
    layout::VList2,
    window::Window
};

enum Message {
    None,
    Incr,
}

impl From<NoResponse> for Message {
    fn from(_: NoResponse) -> Self {
        Message::None
    }
}

fn main() {
    // TODO: need a handler with state (the counter) and ability to connect to widgets
    // (write new label to the `Text` field).
    // How to access widgets? Use index in tuple? Can we name?
    let mut window = Window::new(
        VList2::new(
            Text::from("0"),
            TextButton::new(
                "increment",
                || Message::Incr
            )
        )
    );
    
    window.display();
}
