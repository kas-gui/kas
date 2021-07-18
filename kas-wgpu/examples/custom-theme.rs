// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Custom theme demo
#![cfg_attr(feature = "gat", feature(generic_associated_types))]

use std::cell::Cell;

use kas::draw::color::Rgba;
use kas::draw::*;
use kas::prelude::*;
use kas::widget::*;
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
    static BACKGROUND: Cell<Rgba> = Cell::new(Rgba::grey(1.0));
}

impl<DS: DrawableShared> Theme<DS> for CustomTheme
where
    DS::Draw: DrawableRounded,
{
    type Config = kas_theme::Config;
    type Window = <FlatTheme as Theme<DS>>::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = <FlatTheme as Theme<DS>>::DrawHandle;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = <FlatTheme as Theme<DS>>::DrawHandle<'a>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        Theme::<DS>::config(&self.inner)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        Theme::<DS>::apply_config(&mut self.inner, config)
    }

    fn init(&mut self, shared: &mut DrawShared<DS>) {
        self.inner.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        Theme::<DS>::new_window(&self.inner, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        Theme::<DS>::update_window(&self.inner, window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        shared: &mut DrawShared<DS>,
        draw: Draw<DS::Draw>,
        window: &mut Self::Window,
    ) -> Self::DrawHandle {
        Theme::<DS>::draw_handle(&self.inner, shared, draw, window)
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut DrawShared<DS>,
        draw: Draw<'a, DS::Draw>,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle<'a> {
        Theme::<DS>::draw_handle(&self.inner, shared, draw, window)
    }

    fn clear_color(&self) -> Rgba {
        BACKGROUND.with(|b| b.get())
    }
}

impl ThemeApi for CustomTheme {
    fn set_font_size(&mut self, size: f32) -> TkAction {
        self.inner.set_font_size(size)
    }

    fn list_schemes(&self) -> Vec<&str> {
        self.inner.list_schemes()
    }

    fn set_scheme(&mut self, scheme: &str) -> TkAction {
        self.inner.set_scheme(scheme)
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
            #[widget(row=0, col=1)] _ = TextButton::new_msg("&White", Item::White),
            #[widget(row=1, col=2)] _ = TextButton::new_msg("&Red", Item::Red),
            #[widget(row=2, col=1)] _ = TextButton::new_msg("&Yellow", Item::Yellow),
            #[widget(row=1, col=0)] _ = TextButton::new_msg("&Green", Item::Green),
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
                    -> Response<VoidMsg>
                {
                    match item {
                        Item::White => BACKGROUND.with(|b| b.set(Rgba::WHITE)),
                        Item::Red => BACKGROUND.with(|b| b.set(Rgba::rgb(0.9, 0.2, 0.2))),
                        Item::Green => BACKGROUND.with(|b| b.set(Rgba::rgb(0.2, 0.9, 0.2))),
                        Item::Yellow => BACKGROUND.with(|b| b.set(Rgba::rgb(0.9, 0.9, 0.2))),
                    };
                    Response::None
                }
            }
        },
    );

    kas_wgpu::Toolkit::new(theme)?.with(window)?.run()
}
