// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom theme demo
#![feature(proc_macro_hygiene)]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]

use std::cell::Cell;

use kas::draw::Colour;
use kas::event::{Manager, VoidMsg, VoidResponse};
use kas::geom::Rect;
use kas::macros::{make_widget, VoidMsg};
use kas::theme::{Theme, ThemeAction, ThemeApi};
use kas::widget::*;

use kas_wgpu::draw::*;
use kas_wgpu::glyph::Font;
use kas_wgpu::theme::ShadedTheme;

/// A demo theme
///
/// We set a custom background colour and use `ShadedTheme` for everything else.
pub struct CustomTheme {
    inner: ShadedTheme,
}

impl CustomTheme {
    /// Construct
    pub fn new() -> Self {
        CustomTheme {
            inner: ShadedTheme::new(),
        }
    }
}

// manual impl because derive macro applies incorrect bounds
impl Clone for CustomTheme {
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

impl Theme<DrawPipe> for CustomTheme {
    type Window = <ShadedTheme as Theme<DrawPipe>>::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <ShadedTheme as Theme<DrawPipe>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = <ShadedTheme as Theme<DrawPipe>>::DrawHandle<'a>;

    fn new_window(&self, draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        Theme::<DrawPipe>::new_window(&self.inner, draw, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        Theme::<DrawPipe>::update_window(&self.inner, window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut DrawPipe,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        self.inner.draw_handle(draw, window, rect)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut DrawPipe,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        self.inner.draw_handle(draw, window, rect)
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

impl ThemeApi for CustomTheme {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        ThemeApi::set_font_size(&mut self.inner, size)
    }

    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        ThemeApi::set_colours(&mut self.inner, scheme)
    }
}

#[derive(Clone, Debug, VoidMsg)]
enum Item {
    White,
    Red,
    Green,
    Yellow,
}

fn main() -> Result<(), kas_wgpu::Error> {
    env_logger::init();

    let widgets = make_widget! {
        #[widget]
        #[layout(grid)]
        #[handler(msg = Item)]
        struct {
            #[widget(row=1, col=1)] _ = Label::from("Custom theme demo\nChoose your colour!"),
            #[widget(row=0, col=1)] _ = TextButton::new("White", Item::White),
            #[widget(row=1, col=2)] _ = TextButton::new("Red", Item::Red),
            #[widget(row=2, col=1)] _ = TextButton::new("Yellow", Item::Yellow),
            #[widget(row=1, col=0)] _ = TextButton::new("Green", Item::Green),
        }
    };

    let mut theme = CustomTheme::new();
    let _ = theme.set_colours("light");

    let window = Window::new(
        "Theme demo",
        make_widget! {
            #[widget]
            #[layout(single)]
            #[handler(msg = VoidMsg)]
            struct {
                #[widget(handler = handler)] _ = widgets,
            }
            impl {
                fn handler(&mut self, _: &mut Manager, item: Item)
                    -> VoidResponse
                {
                    match item {
                        Item::White => BACKGROUND.with(|b| b.set(Colour::grey(1.0))),
                        Item::Red => BACKGROUND.with(|b| b.set(Colour::new(0.9, 0.2, 0.2))),
                        Item::Green => BACKGROUND.with(|b| b.set(Colour::new(0.2, 0.9, 0.2))),
                        Item::Yellow => BACKGROUND.with(|b| b.set(Colour::new(0.9, 0.9, 0.2))),
                    };
                    VoidResponse::None
                }
            }
        },
    );

    let mut toolkit = kas_wgpu::Toolkit::new(theme)?;
    toolkit.add(window)?;
    toolkit.run()
}
