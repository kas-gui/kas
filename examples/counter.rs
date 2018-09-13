//! Counter example (simple button)

#[macro_use]
extern crate mygui;

use mygui::event::{NoResponse};
use mygui::widget::{
    canvas::Text,
    control::TextButton,
    window::SimpleWindow
};

use mygui::toolkit::{
    Toolkit,
    gtk::{GtkToolkit, Error}
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

// impl<B: Handler<Response = Message>> Handler for WindowInner<B> {
//     type Response = NoResponse;
//     
//     fn handle(&mut self, ev: event::Event) -> Self::Response {
//         match_event_widget!(ev;
//             display => self.display.handle(ev).into(),
//             button => {
//                 match button.handle(ev) {
//                     Message::None => {},
//                     Message::Incr => {
//                         self.counter += 1;
//                         println!("counter: {}", self.counter);
//                         self.display.set_text(self.counter.to_string());
//                     }
//                 }
//                 NoResponse::None
//             },
//         )
//     }
// }

fn main() -> Result<(), Error> {
    let window = SimpleWindow::new(   // construct with default state and handler
        make_layout!(vertical;
            display D: Text::from("0"),
            button B: TextButton::new("increment", || Message::Incr);
            counter C: 0;
        )
    );
    
    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(&window);
    toolkit.main();
    Ok(())
}
