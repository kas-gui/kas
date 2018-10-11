//! Simple calculator example (lots of buttons, grid layout)

use std::num::ParseFloatError;
use std::str::FromStr;

use mygui::make_widget;
use mygui::event::{NoResponse};
use mygui::widget::{
    canvas::Text,
    control::TextButton,
    window::SimpleWindow
};

use mygui::toolkit::Toolkit;
use mygui_gtk::{GtkToolkit, Error};

#[derive(Clone, Debug, PartialEq)]
enum Key {
    None,
    Clear,
    Divide,
    Multiply,
    Subtract,
    Add,
    Equals,
    Char(u8),  // char in range 0..255
}

impl From<NoResponse> for Key {
    fn from(_: NoResponse) -> Self {
        Key::None
    }
}

fn main() -> Result<(), Error> {
    // TODO: fix grid
    let buttons = make_widget!(grid<C CM, D DM, M MM, S SM,
            N7 N7M, N8 N8M, N9 N9M, A AM,
            N4 N4M, N5 N5M, N6 N6M,
            N1 N1M, N2 N2M, N3 N3M, E EM,
            N0 N0M, P PM>;
        clear: C = TextButton::new("clear", || Key::Clear), // start row
        divide: D = TextButton::new("÷", || Key::Divide),
        multiply: M = TextButton::new("×", || Key::Multiply),
        subtract: S = TextButton::new("−", || Key::Subtract),
        num7: N7 = TextButton::new("7", || Key::Char(48 + 7)), // start row
        num8: N8 = TextButton::new("8", || Key::Char(48 + 8)),
        num9: N9 = TextButton::new("9", || Key::Char(48 + 9)),
        add: A = TextButton::new("+", || Key::Add), // two row span
        num4: N4 = TextButton::new("4", || Key::Char(48 + 4)), // start row
        num5: N5 = TextButton::new("5", || Key::Char(48 + 5)),
        num6: N6 = TextButton::new("6", || Key::Char(48 + 6)),
        num1: N1 = TextButton::new("1", || Key::Char(48 + 1)), // start row
        num2: N2 = TextButton::new("2", || Key::Char(48 + 2)),
        num3: N3 = TextButton::new("3", || Key::Char(48 + 3)),
        equals: E = TextButton::new("=", || Key::Equals),   // two row span
        num0: N0 = TextButton::new("0", || Key::Char(48 + 0)),  // start row; two column span
        decimal: P = TextButton::new(".", || Key::Char(46));;
        Key);
    
    let window = SimpleWindow::new(   // construct with default state and handler
        make_widget!(vertical<BS[Key]>; self, tk, msg;
            // state: Text = Text::from("0") => msg,
            // buf: Text = Text::from("") => msg,
            display: Text = Text::from("0") => msg,
            buttons: BS = buttons => {
                if self.calc.handle(msg) {
                    // self.state.set_text(tk, &self.calc.state_str());
                    // self.buf.set_text(tk, &self.calc.line_buf);
                    self.display.set_text(tk, &self.calc.display());
                }
                NoResponse::None
            };
            calc: Calculator = Calculator::new();
            NoResponse
        )
    );
    
    let mut toolkit = GtkToolkit::new()?;
    toolkit.add(&window);
    toolkit.main();
    Ok(())
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
            Err(e) => format!("{}", e)
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
            op @ Divide | op @ Multiply | op @ Subtract | op @ Add => {
                self.do_op(op)
            }
            Equals => {
                self.do_op(None)
            }
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
                        _ => panic!("unexpected op")    // program error
                    }
                }
                e @ Err(_) => e,
            };
        }
        
        self.op = next_op;
        true
    }
}
