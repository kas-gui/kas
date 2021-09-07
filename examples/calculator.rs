// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple calculator example (lots of buttons, grid layout)

use std::num::ParseFloatError;
use std::str::FromStr;

use kas::class::HasString;
use kas::event::VirtualKeyCode as VK;
use kas::event::{Manager, Response, VoidMsg};
use kas::macros::{make_widget, VoidMsg};
use kas::widgets::{EditBox, TextButton, Window};

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

fn main() -> Result<(), kas::shell::Error> {
    env_logger::init();

    let buttons = make_widget! {
        #[layout(grid)]
        #[handler(msg = Key)]
        #[widget(config=noauto)]
        struct {
            // Buttons get keyboard bindings through the "&" item (e.g. "&1"
            // binds both main and numpad 1 key) and via `with_keys`.
            #[widget(col = 0, row = 0)]
            _ = TextButton::new_msg("&clear", Key::Clear).with_keys(&[VK::Delete]),
            #[widget(col = 1, row = 0)]
            _ = TextButton::new_msg("&÷", Key::Divide).with_keys(&[VK::Slash]),
            #[widget(col = 2, row = 0)]
            _ = TextButton::new_msg("&×", Key::Multiply).with_keys(&[VK::Asterisk]),
            #[widget(col = 3, row = 0)]
            _ = TextButton::new_msg("&−", Key::Subtract),
            #[widget(col = 0, row = 1)]
            _ = TextButton::new_msg("&7", Key::Char('7')),
            #[widget(col = 1, row = 1)]
            _ = TextButton::new_msg("&8", Key::Char('8')),
            #[widget(col = 2, row = 1)]
            _ = TextButton::new_msg("&9", Key::Char('9')),
            #[widget(col = 3, row = 1, rspan = 2, align = stretch)]
            _ = TextButton::new_msg("&+", Key::Add),
            #[widget(col = 0, row = 2)]
            _ = TextButton::new_msg("&4", Key::Char('4')),
            #[widget(col = 1, row = 2)]
            _ = TextButton::new_msg("&5", Key::Char('5')),
            #[widget(col = 2, row = 2)]
            _ = TextButton::new_msg("&6", Key::Char('6')),
            #[widget(col = 0, row = 3)]
            _ = TextButton::new_msg("&1", Key::Char('1')),
            #[widget(col = 1, row = 3)]
            _ = TextButton::new_msg("&2", Key::Char('2')),
            #[widget(col = 2, row = 3)]
            _ = TextButton::new_msg("&3", Key::Char('3')),
            #[widget(col = 3, row = 3, rspan = 2, align = stretch)]
            _ = TextButton::new_msg("&=", Key::Equals).with_keys(&[VK::Return, VK::NumpadEnter]),
            #[widget(col = 0, row = 4, cspan = 2)]
            _ = TextButton::new_msg("&0", Key::Char('0')),
            #[widget(col = 2, row = 4)]
            _ = TextButton::new_msg("&.", Key::Char('.')),
        }
        impl kas::WidgetConfig {
            fn configure(&mut self, mgr: &mut Manager) {
                // Enable key bindings without Alt held:
                mgr.enable_alt_bypass(true);
            }
        }
    };
    let content = make_widget! {
        #[layout(column)]
        #[handler(msg = VoidMsg)]
        struct {
            #[widget] display: impl HasString = EditBox::new("0").editable(false).multi_line(true),
            #[widget(handler = handle_button)] buttons -> Key = buttons,
            calc: Calculator = Calculator::new(),
        }
        impl {
            fn handle_button(&mut self, mgr: &mut Manager, msg: Key) -> Response<VoidMsg> {
                if self.calc.handle(msg) {
                    *mgr |= self.display.set_string(self.calc.display());
                }
                Response::None
            }
        }
    };
    let window = Window::new("Calculator", content);

    let theme = kas::theme::ShadedTheme::new().with_font_size(16.0);
    kas::shell::Toolkit::new(theme)?.with(window)?.run()
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
