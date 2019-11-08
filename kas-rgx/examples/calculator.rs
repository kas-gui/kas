// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple calculator example (lots of buttons, grid layout)
#![feature(proc_macro_hygiene)]

use std::num::ParseFloatError;
use std::str::FromStr;

use kas::class::HasText;
use kas::event::Response;
use kas::event::VirtualKeyCode as VK;
use kas::macros::make_widget;
use kas::widget::{Entry, TextButton, Window};
use kas::TkWindow;

fn main() -> Result<(), winit::error::OsError> {
    let buttons = make_widget! {
        container(grid) => VK;
        struct {
            #[widget(col = 0, row = 0)] _ = TextButton::from_keys("clear", &[VK::Delete]),
            #[widget(col = 1, row = 0)] _ = TextButton::from_keys("÷", &[VK::Divide, VK::Slash]),
            #[widget(col = 2, row = 0)] _ = TextButton::from_keys("×", &[VK::Multiply]),
            #[widget(col = 3, row = 0)] _ = TextButton::from_keys("−", &[VK::Subtract]),
            #[widget(col = 0, row = 1)] _ = TextButton::from_keys("7", &[VK::Key7, VK::Numpad7]),
            #[widget(col = 1, row = 1)] _ = TextButton::from_keys("8", &[VK::Key8, VK::Numpad8]),
            #[widget(col = 2, row = 1)] _ = TextButton::from_keys("9", &[VK::Key9, VK::Numpad9]),
            #[widget(col = 3, row = 1, rspan = 2)] _ = TextButton::from_keys("+", &[VK::Add]),
            #[widget(col = 0, row = 2)] _ = TextButton::from_keys("4", &[VK::Key4, VK::Numpad4]),
            #[widget(col = 1, row = 2)] _ = TextButton::from_keys("5", &[VK::Key5, VK::Numpad5]),
            #[widget(col = 2, row = 2)] _ = TextButton::from_keys("6", &[VK::Key6, VK::Numpad6]),
            #[widget(col = 0, row = 3)] _ = TextButton::from_keys("1", &[VK::Key1, VK::Numpad1]),
            #[widget(col = 1, row = 3)] _ = TextButton::from_keys("2", &[VK::Key2, VK::Numpad2]),
            #[widget(col = 2, row = 3)] _ = TextButton::from_keys("3", &[VK::Key3, VK::Numpad3]),
            #[widget(col = 3, row = 3, rspan = 2)] _ = TextButton::from_keys("=",
                &[VK::Equals, VK::Return, VK::NumpadEnter, VK::NumpadEquals]),
            #[widget(col = 0, row = 4, cspan = 2)] _ = TextButton::from_keys("0",
                &[VK::Key0, VK::Numpad0]),
            #[widget(col = 2, row = 4)] _ = TextButton::from_keys(".", &[VK::Period]),
        }
    };
    let content = make_widget! {
        container(vertical) => ();
        struct {
            #[widget] display: impl HasText = Entry::new("0").editable(false),
            #[widget(handler = handle_button)] buttons -> VK = buttons,
            calc: Calculator = Calculator::new(),
        }
        impl {
            fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: VK) -> Response<()> {
                if self.calc.handle(msg) {
                    self.display.set_text(tk, self.calc.display());
                }
                Response::None
            }
        }
    };
    let window = Window::new(content);

    let mut toolkit = kas_rgx::Toolkit::new();
    toolkit.add(window)?;
    toolkit.run()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    None,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Clone, Debug)]
struct Calculator {
    state: Result<f64, ParseFloatError>,
    op: Op,
    line_buf: String,
}

impl Calculator {
    fn new() -> Calculator {
        Calculator {
            state: Ok(0.0),
            op: Op::None,
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
        // Single-line display:
        /*if self.line_buf.is_empty() {
            self.state_str()
        } else {
            self.line_buf.clone()
        }*/

        // Multi-line display:
        let op = match self.op {
            Op::None => "",
            Op::Add => "+",
            Op::Subtract => "−",
            Op::Multiply => "×",
            Op::Divide => "÷",
        };
        format!("{}\n{}\n{}", self.state_str(), op, &self.line_buf)
    }

    // return true if display changes
    fn handle(&mut self, key: VK) -> bool {
        match key {
            VK::Delete => {
                self.state = Ok(0.0);
                self.op = Op::None;
                self.line_buf.clear();
                true
            }
            VK::Divide => self.do_op(Op::Divide),
            VK::Multiply => self.do_op(Op::Multiply),
            VK::Subtract => self.do_op(Op::Subtract),
            VK::Add => self.do_op(Op::Add),
            VK::Equals => self.do_op(Op::None),
            VK::Period => self.push_char('.'),
            VK::Key0 => self.push_char('0'),
            VK::Key1 => self.push_char('1'),
            VK::Key2 => self.push_char('2'),
            VK::Key3 => self.push_char('3'),
            VK::Key4 => self.push_char('4'),
            VK::Key5 => self.push_char('5'),
            VK::Key6 => self.push_char('6'),
            VK::Key7 => self.push_char('7'),
            VK::Key8 => self.push_char('8'),
            VK::Key9 => self.push_char('9'),
            _ => unreachable!(),
        }
    }

    fn do_op(&mut self, next_op: Op) -> bool {
        if self.line_buf.is_empty() {
            self.op = next_op;
            return true;
        }

        let line = f64::from_str(&self.line_buf);
        self.line_buf.clear();

        if self.op == Op::None {
            self.state = line;
        } else if let Ok(x) = self.state {
            self.state = match line {
                Ok(y) => {
                    match self.op {
                        Op::Divide => Ok(x / y),
                        Op::Multiply => Ok(x * y),
                        Op::Subtract => Ok(x - y),
                        Op::Add => Ok(x + y),
                        _ => panic!("unexpected op"), // program error
                    }
                }
                e @ Err(_) => e,
            };
        }

        self.op = next_op;
        true
    }

    fn push_char(&mut self, c: char) -> bool {
        self.line_buf.push(c);
        true
    }
}
