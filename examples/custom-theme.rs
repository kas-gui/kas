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
use kas::theme::Theme;
use kas::widgets::*;

use kas::theme::FlatTheme;

/// A demo theme
///
/// We set a custom background colour and use `FlatTheme` for everything else.
#[derive(Clone, Default)]
pub struct CustomTheme {
    inner: FlatTheme,
}

thread_local! {
    // Cell is not thread-safe. Lazy way to solve that :-D
    static BACKGROUND: Cell<Rgba> = Cell::new(Rgba::grey(1.0));
}

impl<DS: DrawSharedImpl> Theme<DS> for CustomTheme
where
    DS::Draw: DrawRoundedImpl,
{
    type Config = kas::theme::Config;
    type Window = <FlatTheme as Theme<DS>>::Window;

    #[cfg(not(feature = "gat"))]
    type Draw = <FlatTheme as Theme<DS>>::Draw;
    #[cfg(feature = "gat")]
    type Draw<'a> = <FlatTheme as Theme<DS>>::Draw<'a>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        Theme::<DS>::config(&self.inner)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        Theme::<DS>::apply_config(&mut self.inner, config)
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        self.inner.init(shared);
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        Theme::<DS>::new_window(&self.inner, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        Theme::<DS>::update_window(&self.inner, window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw(
        &self,
        draw: DrawIface<DS>,
        ev: &mut EventState,
        window: &mut Self::Window,
    ) -> Self::Draw {
        Theme::<DS>::draw(&self.inner, draw, ev, window)
    }
    #[cfg(feature = "gat")]
    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        window: &'a mut Self::Window,
    ) -> Self::Draw<'a> {
        Theme::<DS>::draw(&self.inner, draw, ev, window)
    }

    fn clear_color(&self) -> Rgba {
        BACKGROUND.with(|b| b.get())
    }
}

impl ThemeControl for CustomTheme {
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

#[derive(Clone, Debug)]
enum Item {
    White,
    Red,
    Green,
    Yellow,
}

fn main() -> kas::shell::Result<()> {
    env_logger::init();

    let theme = CustomTheme::default();

    impl_scope! {
        #[widget{
        layout = grid: {
            1, 1: "Custom theme demo\nChoose your colour!";
            0, 1: TextButton::new_msg("&White", Item::White);
            1, 2: TextButton::new_msg("&Red", Item::Red);
            2, 1: TextButton::new_msg("&Yellow", Item::Yellow);
            1, 0: TextButton::new_msg("&Green", Item::Green);
        };
        }]
        #[derive(Debug, Default)]
        struct Demo(widget_core!());
        impl Widget for Self {
            fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
                if let Some(item) = mgr.try_pop_msg::<Item>() {
                    match item {
                        Item::White => BACKGROUND.with(|b| b.set(Rgba::WHITE)),
                        Item::Red => BACKGROUND.with(|b| b.set(Rgba::rgb(0.9, 0.2, 0.2))),
                        Item::Green => BACKGROUND.with(|b| b.set(Rgba::rgb(0.2, 0.9, 0.2))),
                        Item::Yellow => BACKGROUND.with(|b| b.set(Rgba::rgb(0.9, 0.9, 0.2))),
                    }
                }
            }
        }
        impl Window for Self {
            fn title(&self) -> &str { "Theme demo" }
        }
    }

    kas::shell::Toolkit::new(theme)?
        .with(Demo::default())?
        .run()
}
