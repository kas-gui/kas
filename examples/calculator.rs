// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple calculator example (lots of buttons, grid layout)

use std::num::ParseFloatError;
use std::str::FromStr;

use kas::event::NamedKey;
use kas::prelude::*;
use kas::widgets::{AccessLabel, Adapt, Button, EditBox};

type Key = kas::event::Key<kas::event::SmolStr>;

fn key_button(label: &str) -> Button<AccessLabel> {
    let string = AccessString::from(label);
    let key = string.key().unwrap().clone();
    Button::label_msg(string, key)
}
fn key_button_with(label: &str, key: Key) -> Button<AccessLabel> {
    Button::label_msg(label, key.clone()).with_access_key(key)
}

fn calc_ui() -> Window<()> {
    // We could use kas::widget::Text, but EditBox looks better.
    let display = EditBox::string(|calc: &Calculator| calc.display())
        .with_multi_line(true)
        .with_lines(3, 3)
        .with_width_em(5.0, 10.0);

    // We use map_any to avoid passing input data (not wanted by buttons):
    let buttons = kas::grid! {
        // Key bindings: C, Del
        (0, 0) => Button::label_msg("&clear", Key::Named(NamedKey::Clear))
            .with_access_key(NamedKey::Delete.into()),
        // Widget is hidden but has key binding.
        // TODO(opt): exclude from layout & drawing.
        (0, 0) => key_button_with("", NamedKey::Backspace.into()),
        (1, 0) => key_button_with("&÷", Key::Character("/".into())),
        (2, 0) => key_button_with("&×", Key::Character("*".into())),
        (3, 0) => key_button_with("&−", Key::Character("-".into())),
        (0, 1) => key_button("&7"),
        (1, 1) => key_button("&8"),
        (2, 1) => key_button("&9"),
        (3, 1..3) => key_button("&+"),
        (0, 2) => key_button("&4"),
        (1, 2) => key_button("&5"),
        (2, 2) => key_button("&6"),
        (0, 3) => key_button("&1"),
        (1, 3) => key_button("&2"),
        (2, 3) => key_button("&3"),
        (3, 3..5) => key_button_with("&=", NamedKey::Enter.into()),
        (0..2, 4) => key_button("&0"),
        (2, 4) => key_button("&."),
    }
    .map_any();

    let ui = Adapt::new(kas::column![display, buttons], Calculator::new())
        .on_message(|_, calc, key| calc.handle(key))
        .on_configure(|cx, adapt| {
            cx.disable_nav_focus(true);

            // Enable key bindings without Alt held:
            cx.enable_alt_bypass(adapt.id_ref(), true);
        });

    Window::new(ui, "Calculator")
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = kas_wgpu::ShadedTheme::new().with_font_size(16.0);
    kas::shell::Default::with_theme(theme)
        .build(())?
        .with(calc_ui())
        .run()
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
            Err(e) => format!("{e}"),
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

    fn handle(&mut self, key: Key) {
        match key {
            Key::Named(NamedKey::Clear) | Key::Named(NamedKey::Delete) => {
                self.state = Ok(0.0);
                self.op = Op::None;
                self.line_buf.clear();
            }
            Key::Named(NamedKey::Backspace) => {
                self.line_buf.pop();
            }
            Key::Character(s) if s == "/" => self.do_op(Op::Divide),
            Key::Character(s) if s == "*" => self.do_op(Op::Multiply),
            Key::Character(s) if s == "-" => self.do_op(Op::Subtract),
            Key::Character(s) if s == "+" => self.do_op(Op::Add),
            Key::Named(NamedKey::Enter) => self.do_op(Op::None),
            Key::Character(s) if s.len() == 1 => {
                let c = s.chars().next().unwrap();
                if ('0'..='9').contains(&c) || c == '.' {
                    self.line_buf.push(c);
                }
            }
            _ => (),
        }
    }

    fn do_op(&mut self, next_op: Op) {
        if self.line_buf.is_empty() {
            self.op = next_op;
            return;
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
    }
}
