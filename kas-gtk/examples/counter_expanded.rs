// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Like counter example, but avoiding usage of make_widget
#![feature(unrestricted_attribute_tokens)]
#![feature(proc_macro_hygiene)]

use kas::control::TextButton;
use kas::text::Label;
use kas::event::{Handler, NoResponse};
use kas::macros::{NoResponse, Widget};
use kas::HasText;
use kas::{SimpleWindow, Toolkit, TkWidget, Class, CoreData, Widget};

#[derive(Debug, NoResponse)]
enum Message {
    None,
    Decr,
    Incr,
}

#[widget(class = Class::Container, layout = horizontal)]
#[handler(response = Message, generics = <>
        where D: Handler<Response = Message>, I: Handler<Response = Message>)]
#[derive(Debug, Widget)]
struct Buttons<D: Widget, I: Widget> {
    #[core] core: CoreData,
    #[widget] decr: D,
    #[widget] incr: I,
}

#[widget(class = Class::Container, layout = vertical)]
#[handler(response = NoResponse, generics = <>
        where B: Handler<Response = Message>)]
#[derive(Debug, Widget)]
struct Contents<B: Widget> {
    #[core] core: CoreData,
    #[widget] display: Label,
    #[widget(handler = handle_button)] buttons: B,
    counter: usize,
}

impl<B: Widget> Contents<B> {
    fn handle_button(&mut self, tk: &TkWidget, msg: Message) -> NoResponse {
        match msg {
            Message::None => (),
            Message::Decr => {
                self.counter = self.counter.saturating_sub(1);
                self.display.set_text(tk, &self.counter.to_string());
            }
            Message::Incr => {
                self.counter = self.counter.saturating_add(1);
                self.display.set_text(tk, &self.counter.to_string());
            }
        };
        NoResponse
    }
}


fn main() -> Result<(), kas_gtk::Error> {
    let buttons = Buttons {
        core: CoreData::default(),
        decr: TextButton::new_on("−", || Message::Decr),
        incr: TextButton::new_on("+", || Message::Incr),
    };
    
    let contents = Contents {
        core: CoreData::default(),
        display: Label::from("0"),
        buttons: buttons,
        counter: 0,
    };
    
    let window = SimpleWindow::new(contents);

    let mut toolkit = kas_gtk::Toolkit::new()?;
    toolkit.add(window);
    toolkit.main();
    Ok(())
}
