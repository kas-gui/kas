// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom theme demo
#![feature(proc_macro_hygiene)]

use std::cell::Cell;

use kas::draw::*;
use kas::event::{self, EmptyMsg};
use kas::geom::Size;
use kas::layout::{AxisInfo, SizeRules};
use kas::macros::{make_widget, EmptyMsg};
use kas::widget::*;
use kas::{TkWindow, Widget};

use kas_wgpu::draw::*;
use kas_wgpu::glyph::Font;
use kas_wgpu::SampleTheme;

/// A demo theme
///
/// We set a custom background colour and use `SampleTheme` for everything else.
pub struct ColouredTheme<D> {
    inner: SampleTheme<D>,
}

impl<D> ColouredTheme<D> {
    /// Construct
    pub fn new() -> Self {
        ColouredTheme {
            inner: SampleTheme::new(),
        }
    }
}

// manual impl because derive macro applies incorrect bounds
impl<D> Clone for ColouredTheme<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

thread_local! {
    // Cell is not thread-safe. Lazy way to solve that :-D
    static BACKGROUND: Cell<Colour> = Cell::new(Colour::grey(1.0));
}

impl<D: Draw + DrawText> Theme for ColouredTheme<D> {
    type Draw = D;

    fn set_dpi_factor(&mut self, factor: f32) {
        self.inner.set_dpi_factor(factor)
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        self.inner.get_fonts()
    }

    fn light_direction(&self) -> (f32, f32) {
        self.inner.light_direction()
    }

    fn clear_colour(&self) -> Colour {
        BACKGROUND.with(|b| b.get())
    }

    fn size_rules(&self, draw: &mut D, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        self.inner.size_rules(draw, widget, axis)
    }

    fn inner_margin(&self, widget: &dyn Widget, axis_is_vertical: bool) -> u32 {
        self.inner.inner_margin(widget, axis_is_vertical)
    }

    fn child_margins(&self, widget: &dyn Widget) -> (Size, Size, Size) {
        self.inner.child_margins(widget)
    }

    fn draw(&self, draw: &mut D, ev_mgr: &event::Manager, widget: &dyn kas::Widget) {
        self.inner.draw(draw, ev_mgr, widget)
    }
}

#[derive(Clone, Debug, EmptyMsg)]
enum Item {
    None,
    White,
    Red,
    Green,
    Yellow,
}

fn main() -> Result<(), winit::error::OsError> {
    let widgets = make_widget! {
        container(grid) => Item;
        struct {
            #[widget(row=1, col=1)] _ = Label::from("Custom theme demo\nChoose your colour!"),
            #[widget(row=0, col=1)] _ = TextButton::new("White", Item::White),
            #[widget(row=1, col=2)] _ = TextButton::new("Red", Item::Red),
            #[widget(row=2, col=1)] _ = TextButton::new("Yellow", Item::Yellow),
            #[widget(row=1, col=0)] _ = TextButton::new("Green", Item::Green),
        }
    };

    let theme = ColouredTheme::new();

    let window = Window::new(make_widget! {
        container(single) => EmptyMsg;
        struct {
            #[widget(handler = handler)] _ = widgets,
        }
        impl {
            fn handler(&mut self, _: &mut dyn TkWindow, item: Item)
                -> EmptyMsg
            {
                match item {
                    Item::None => (),
                    Item::White => BACKGROUND.with(|b| b.set(Colour::grey(1.0))),
                    Item::Red => BACKGROUND.with(|b| b.set(Colour::new(0.9, 0.2, 0.2))),
                    Item::Green => BACKGROUND.with(|b| b.set(Colour::new(0.2, 0.9, 0.2))),
                    Item::Yellow => BACKGROUND.with(|b| b.set(Colour::new(0.9, 0.9, 0.2))),
                };
                EmptyMsg
            }
        }
    });

    let mut toolkit = kas_wgpu::Toolkit::new(theme);
    toolkit.add(window)?;
    toolkit.run()
}
