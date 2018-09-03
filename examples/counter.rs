//! Counter example (simple button)

#[macro_use]
extern crate mygui;

use mygui::event::{self, Handler, NoResponse};
use mygui::widget::{
    Widget, CoreData,
    canvas::Text,
    control::TextButton,
    Layout,
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

struct WindowInner<B> {
    core: CoreData,
    display: Text,
    button: B,
    counter: usize,
}

impl_widget_core!(WindowInner<B>, core);
impl_layout!(WindowInner<B: Layout>; vlist(display, button));

impl<B: Handler<Response = Message>> Handler for WindowInner<B> {
    type Response = NoResponse;
    
    fn handle(&mut self, ev: event::Event) -> Self::Response {
        match_event_widget!(ev;
            display => self.display.handle(ev).into(),
            button => {
                match button.handle(ev) {
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

impl<B: Widget+'static> Widget for WindowInner<B> {
    fn len(&self) -> usize { 2 }
    fn get(&self, index: usize) -> Option<&(dyn Widget + 'static)> {
        match index {
            0 => Some(&self.display),
            1 => Some(&self.button),
            _ => None
        }
    }
}


fn main() -> Result<(), Error> {
    let window = SimpleWindow::new(   // construct with default state and handler
        WindowInner {
            core: Default::default(),
            display: Text::from("0"),
            button: TextButton::new("increment", || Message::Incr),
            counter: 0
        });
    
    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
