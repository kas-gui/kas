// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple calculator example (lots of buttons, grid layout)
#![feature(proc_macro_hygiene)]

use std::num::ParseFloatError;
use std::str::FromStr;

use kas::control::TextButton;
use kas::text::Entry;
use kas::macros::make_widget;
use kas::HasText;
use kas::{SimpleWindow, TkWidget};

#[derive(Clone, Debug, PartialEq)]
enum Key {
    Clear,
    Divide,
    Multiply,
    Subtract,
    Add,
    Equals,
    Char(u8), // char in range 0..255
}

fn main() -> Result<(), winit::error::OsError> {
    let buttons = make_widget!{
        container(grid) => Key;
        struct {
            #[widget(col = 0, row = 0)] _ = TextButton::new_on("clear", || Key::Clear),
            #[widget(col = 1, row = 0)] _ = TextButton::new_on("÷", || Key::Divide),
            #[widget(col = 2, row = 0)] _ = TextButton::new_on("×", || Key::Multiply),
            #[widget(col = 3, row = 0)] _ = TextButton::new_on("−", || Key::Subtract),
            #[widget(col = 0, row = 1)] _ = TextButton::new_on("7", || Key::Char(48 + 7)),
            #[widget(col = 1, row = 1)] _ = TextButton::new_on("8", || Key::Char(48 + 8)),
            #[widget(col = 2, row = 1)] _ = TextButton::new_on("9", || Key::Char(48 + 9)),
            #[widget(col = 3, row = 1, rspan = 2)] _ = TextButton::new_on("+", || Key::Add),
            #[widget(col = 0, row = 2)] _ = TextButton::new_on("4", || Key::Char(48 + 4)),
            #[widget(col = 1, row = 2)] _ = TextButton::new_on("5", || Key::Char(48 + 5)),
            #[widget(col = 2, row = 2)] _ = TextButton::new_on("6", || Key::Char(48 + 6)),
            #[widget(col = 0, row = 3)] _ = TextButton::new_on("1", || Key::Char(48 + 1)),
            #[widget(col = 1, row = 3)] _ = TextButton::new_on("2", || Key::Char(48 + 2)),
            #[widget(col = 2, row = 3)] _ = TextButton::new_on("3", || Key::Char(48 + 3)),
            #[widget(col = 3, row = 3, rspan = 2)] _ = TextButton::new_on("=", || Key::Equals),
            #[widget(col = 0, row = 4, cspan = 2)] _ = TextButton::new_on("0", || Key::Char(48 + 0)),
            #[widget(col = 2, row = 4)] _ = TextButton::new_on(".", || Key::Char(46)),
        }
    };
    let content = make_widget!{
        container(vertical) => ();
        struct {
            // #[widget] state: Label = Label::from("0"),
            // #[widget] buf: Label = Label::new() ,
            #[widget] display: impl HasText = Entry::new("0").editable(false),
            #[widget(handler = handle_button)] buttons -> Key = buttons,
            calc: Calculator = Calculator::new(),
        }
        impl {
            fn handle_button(&mut self, tk: &mut dyn TkWidget, msg: Key) -> Response<()> {
                if self.calc.handle(msg) {
                    // self.state.set_text(tk, &self.calc.state_str());
                    // self.buf.set_text(tk, &self.calc.line_buf);
                    self.display.set_text(tk, self.calc.display());
                }
                Response::None
            }
        }
    };
    let window = SimpleWindow::new(content);

    let mut toolkit = kas_rgx::Toolkit::new();
    toolkit.add(window);
    toolkit.run()
}

#[derive(Clone, Debug)]
struct Calculator {
    state: Result<f64, ParseFloatError>,
    op: Key,
    line_buf: String,
}

impl Calculator {
    fn new() -> Calculator {
        Calculator {
            state: Ok(0.0),
            op: Key::None,
            line_buf: String::new(),
        }
    }

    fn state_str(&self) -> String {
        match &self.state {
            Ok(x) => x.to_string(),
            Err(e) => format!("{}", e),
        }
    }

    // alternative, single line display
    #[allow(unused)]
    fn display(&self) -> String {
        if self.line_buf.is_empty() {
            self.state_str()
        } else {
            self.line_buf.clone()
        }
    }

    // return true if display changes
    fn handle(&mut self, key: Key) -> bool {
        use self::Key::*;
        match key {
            None => false,
            Clear => {
                self.state = Ok(0.0);
                self.op = None;
                self.line_buf.clear();
                true
            }
            op @ Divide | op @ Multiply | op @ Subtract | op @ Add => self.do_op(op),
            Equals => self.do_op(None),
            Char(c) => {
                self.line_buf.push(char::from(c));
                true
            }
        }
    }

    fn do_op(&mut self, next_op: Key) -> bool {
        if self.line_buf.is_empty() {
            self.op = next_op;
            return false;
        }

        let line = f64::from_str(&self.line_buf);
        self.line_buf.clear();

        if self.op == Key::None {
            self.state = line;
        } else if let Ok(x) = self.state {
            self.state = match line {
                Ok(y) => {
                    use self::Key::*;
                    match self.op {
                        Divide => Ok(x / y),
                        Multiply => Ok(x * y),
                        Subtract => Ok(x - y),
                        Add => Ok(x + y),
                        _ => panic!("unexpected op"), // program error
                    }
                }
                e @ Err(_) => e,
            };
        }

        self.op = next_op;
        true
    }
}
