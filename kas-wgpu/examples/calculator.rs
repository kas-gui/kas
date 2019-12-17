// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple calculator example (lots of buttons, grid layout)
#![recursion_limit = "1024"]

use std::num::ParseFloatError;
use std::str::FromStr;

use kas::class::HasText;
use kas::event::VirtualKeyCode as VK;
use kas::event::{Response, VoidMsg};
use kas::macros::VoidMsg;
use kas::widget::{EditBox, TextButton, Window};
use kas::{make_widget, TkWindow};

#[derive(Clone, Debug, VoidMsg)]
enum Key {
    Clear,
    Divide,
    Multiply,
    Subtract,
    Add,
    Equals,
    Char(char),
}

fn main() -> Result<(), winit::error::OsError> {
    let buttons = make_widget! {
        grid => Key;
        struct {
            #[widget(col = 0, row = 0)]
            _ = TextButton::new("clear", Key::Clear).with_keys(&[VK::Delete]),
            #[widget(col = 1, row = 0)]
            _ = TextButton::new("÷", Key::Divide).with_keys(&[VK::Divide, VK::Slash]),
            #[widget(col = 2, row = 0)]
            _ = TextButton::new("×", Key::Multiply).with_keys(&[VK::Multiply]),
            #[widget(col = 3, row = 0)]
            _ = TextButton::new("−", Key::Subtract).with_keys(&[VK::Subtract]),
            #[widget(col = 0, row = 1)]
            _ = TextButton::new("7", Key::Char('7')).with_keys(&[VK::Key7, VK::Numpad7]),
            #[widget(col = 1, row = 1)]
            _ = TextButton::new("8", Key::Char('8')).with_keys(&[VK::Key8, VK::Numpad8]),
            #[widget(col = 2, row = 1)]
            _ = TextButton::new("9", Key::Char('9')).with_keys(&[VK::Key9, VK::Numpad9]),
            #[widget(col = 3, row = 1, rspan = 2)]
            _ = TextButton::new("+", Key::Add).with_keys(&[VK::Add]),
            #[widget(col = 0, row = 2)]
            _ = TextButton::new("4", Key::Char('4')).with_keys(&[VK::Key4, VK::Numpad4]),
            #[widget(col = 1, row = 2)]
            _ = TextButton::new("5", Key::Char('5')).with_keys(&[VK::Key5, VK::Numpad5]),
            #[widget(col = 2, row = 2)]
            _ = TextButton::new("6", Key::Char('6')).with_keys(&[VK::Key6, VK::Numpad6]),
            #[widget(col = 0, row = 3)]
            _ = TextButton::new("1", Key::Char('1')).with_keys(&[VK::Key1, VK::Numpad1]),
            #[widget(col = 1, row = 3)]
            _ = TextButton::new("2", Key::Char('2')).with_keys(&[VK::Key2, VK::Numpad2]),
            #[widget(col = 2, row = 3)]
            _ = TextButton::new("3", Key::Char('3')).with_keys(&[VK::Key3, VK::Numpad3]),
            #[widget(col = 3, row = 3, rspan = 2)]
            _ = TextButton::new("=", Key::Equals)
                .with_keys(&[VK::Equals, VK::Return, VK::NumpadEnter, VK::NumpadEquals]),
            #[widget(col = 0, row = 4, cspan = 2)]
            _ = TextButton::new("0", Key::Char('0')).with_keys(&[VK::Key0, VK::Numpad0]),
            #[widget(col = 2, row = 4)]
            _ = TextButton::new(".", Key::Char('.')).with_keys(&[VK::Period]),
        }
    };
    let content = make_widget! {
        vertical => VoidMsg;
        struct {
            #[widget] display: impl HasText = EditBox::new("0").editable(false).multi_line(true),
            #[widget(handler = handle_button)] buttons -> Key = buttons,
            calc: Calculator = Calculator::new(),
        }
        impl {
            fn handle_button(&mut self, tk: &mut dyn TkWindow, msg: Key) -> Response<VoidMsg> {
                if self.calc.handle(msg) {
                    self.display.set_text(tk, self.calc.display());
                }
                Response::None
            }
        }
    };
    let window = Window::new("Calculator", content);

    let mut theme = kas_wgpu::SampleTheme::new();
    theme.set_font_size(24.0);
    let mut toolkit = kas_wgpu::Toolkit::new(theme);
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
    fn handle(&mut self, key: Key) -> bool {
        match key {
            Key::Clear => {
                self.state = Ok(0.0);
                self.op = Op::None;
                self.line_buf.clear();
                true
            }
            Key::Divide => self.do_op(Op::Divide),
            Key::Multiply => self.do_op(Op::Multiply),
            Key::Subtract => self.do_op(Op::Subtract),
            Key::Add => self.do_op(Op::Add),
            Key::Equals => self.do_op(Op::None),
            Key::Char(c) => self.push_char(c),
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
