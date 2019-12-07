// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom theme demo
#![feature(proc_macro_hygiene)]

use std::cell::Cell;

use kas::draw::Colour;
use kas::event::EmptyMsg;
use kas::macros::{make_widget, EmptyMsg};
use kas::theme::Theme;
use kas::widget::*;
use kas::TkWindow;

use kas_wgpu::draw::*;
use kas_wgpu::glyph::Font;
use kas_wgpu::SampleTheme;

/// A demo theme
///
/// We set a custom background colour and use `SampleTheme` for everything else.
pub struct ColouredTheme {
    inner: SampleTheme,
}

impl ColouredTheme {
    /// Construct
    pub fn new() -> Self {
        ColouredTheme {
            inner: SampleTheme::new(),
        }
    }
}

// manual impl because derive macro applies incorrect bounds
impl Clone for ColouredTheme {
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

impl Theme<DrawPipe> for ColouredTheme {
    type Window = <SampleTheme as Theme<DrawPipe>>::Window;
    type DrawHandle = <SampleTheme as Theme<DrawPipe>>::DrawHandle;

    fn new_window(&self, draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        Theme::<DrawPipe>::new_window(&self.inner, draw, dpi_factor)
    }

    unsafe fn draw_handle(
        &self,
        draw: &mut DrawPipe,
        theme_window: &mut Self::Window,
    ) -> Self::DrawHandle {
        self.inner.draw_handle(draw, theme_window)
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        Theme::<DrawPipe>::get_fonts(&self.inner)
    }

    fn light_direction(&self) -> (f32, f32) {
        Theme::<DrawPipe>::light_direction(&self.inner)
    }

    fn clear_colour(&self) -> Colour {
        BACKGROUND.with(|b| b.get())
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
        grid => Item;
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
        single => EmptyMsg;
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
