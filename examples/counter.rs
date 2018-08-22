//! Counter example (simple button)

#[macro_use]
extern crate mygui;

use mygui::widget::{
    Widget, WidgetCore,
    canvas::Text,
    control::TextButton,
    event::{self, NoResponse},
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

struct WindowInner<B> {
    display: Text,
    button: B,
    counter: usize,
}

impl_layout!(WindowInner; vertical; display, button);

impl<B> WidgetCore for WindowInner<B> {}

impl<B: Widget<Response = Message>> Widget for WindowInner<B> {
    type Response = NoResponse;
    
    fn handle(&mut self, event: event::Event) -> Self::Response {
        match_event_widget!(event;
            display => self.display.handle(event).into(),
            button => {
                match button.handle(event) {
                    Message::None => {},
                    Message::Incr => {
                        self.counter += 1;
                        self.display.set_text(self.counter.to_string());
                    }
                }
                NoResponse::None
            },
        )
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
