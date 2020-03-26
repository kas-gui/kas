// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom theme demo
#![feature(proc_macro_hygiene)]
#![cfg_attr(feature = "gat", feature(generic_associated_types))]

use std::cell::Cell;

use kas::draw::*;
use kas::event::{Manager, VoidMsg, VoidResponse};
use kas::geom::Rect;
use kas::macros::{make_widget, VoidMsg};
use kas::widget::*;
use kas::{ThemeAction, ThemeApi};
use kas_theme::Theme;

use kas_theme::FlatTheme;

/// A demo theme
///
/// We set a custom background colour and use `FlatTheme` for everything else.
pub struct CustomTheme {
    inner: FlatTheme,
}

impl CustomTheme {
    /// Construct
    pub fn new() -> Self {
        CustomTheme {
            inner: FlatTheme::new(),
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

impl<D: DrawShared + DrawTextShared + 'static> Theme<D> for CustomTheme
where
    D::Draw: DrawRounded + DrawText,
{
    type Window = <FlatTheme as Theme<D>>::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <FlatTheme as Theme<D>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = <FlatTheme as Theme<D>>::DrawHandle<'a>;

    fn init(&mut self, draw: &mut D) {
        self.inner.init(draw);
    }

    fn new_window(&self, draw: &mut D::Draw, dpi_factor: f32) -> Self::Window {
        Theme::<D>::new_window(&self.inner, draw, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        Theme::<D>::update_window(&self.inner, window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        draw: &mut D::Draw,
        window: &mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        Theme::<D>::draw_handle(&self.inner, draw, window, rect)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        Theme::<D>::draw_handle(&self.inner, draw, window, rect)
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
        #[layout(grid)]
        #[handler(msg = Item)]
        struct {
            #[widget(row=1, col=1)] _ = Label::new("Custom theme demo\nChoose your colour!"),
            #[widget(row=0, col=1)] _ = TextButton::new("White", Item::White),
            #[widget(row=1, col=2)] _ = TextButton::new("Red", Item::Red),
            #[widget(row=2, col=1)] _ = TextButton::new("Yellow", Item::Yellow),
            #[widget(row=1, col=0)] _ = TextButton::new("Green", Item::Green),
        }
    };

    let theme = CustomTheme::new();

    let window = Window::new(
        "Theme demo",
        make_widget! {
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
