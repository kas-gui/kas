// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use rusttype::Font;
use std::collections::HashMap;
use std::marker::Unsize;

use crate::{StackDst, Theme, ThemeDst, WindowDst};
use kas::draw::{Colour, DrawHandle};
use kas::geom::Rect;
use kas::{ThemeAction, ThemeApi};

/// Wrapper around mutliple themes, supporting run-time switching
///
/// **Feature gated**: this is only available with feature `stack_dst`.
pub struct MultiTheme<Draw> {
    names: HashMap<String, usize>,
    themes: Vec<StackDst<dyn ThemeDst<Draw>>>,
    active: usize,
}

/// Builder for [`MultiTheme`]
///
/// Construct via [`MultiTheme::builder`].
pub struct MultiThemeBuilder<Draw> {
    names: HashMap<String, usize>,
    themes: Vec<StackDst<dyn ThemeDst<Draw>>>,
}

impl<Draw> MultiTheme<Draw> {
    /// Construct with builder pattern
    pub fn builder() -> MultiThemeBuilder<Draw> {
        MultiThemeBuilder {
            names: HashMap::new(),
            themes: vec![],
        }
    }
}

impl<Draw> MultiThemeBuilder<Draw> {
    /// Add a theme
    pub fn add<S: ToString, U>(mut self, name: S, theme: U) -> Self
    where
        U: Unsize<dyn ThemeDst<Draw>>,
        Box<U>: Unsize<dyn ThemeDst<Draw>>,
    {
        let index = self.themes.len();
        self.names.insert(name.to_string(), index);
        self.themes.push(StackDst::new_or_boxed(theme));
        self
    }

    /// Build
    ///
    /// Fails if no themes were added.
    pub fn try_build(self) -> Result<MultiTheme<Draw>, ()> {
        if self.themes.len() == 0 {
            return Err(());
        }
        Ok(MultiTheme {
            names: self.names,
            themes: self.themes,
            active: 0,
        })
    }

    /// Build
    ///
    /// Panics if no themes were added.
    pub fn build(self) -> MultiTheme<Draw> {
        self.try_build()
            .unwrap_or_else(|_| panic!("MultiThemeBuilder: no themes added"))
    }
}

impl<Draw: 'static> Theme<Draw> for MultiTheme<Draw> {
    type Window = StackDst<dyn WindowDst<Draw>>;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = StackDst<dyn DrawHandle>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = StackDst<dyn DrawHandle + 'a>;

    fn new_window(&self, draw: &mut Draw, dpi_factor: f32) -> Self::Window {
        self.themes[self.active].new_window(draw, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.themes[self.active].update_window(window, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle> {
        self.themes[self.active].draw_handle(draw, window, rect)
    }

    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> StackDst<dyn DrawHandle + 'a> {
        self.themes[self.active].draw_handle(draw, window, rect)
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        self.themes[self.active].get_fonts()
    }

    fn light_direction(&self) -> (f32, f32) {
        self.themes[self.active].light_direction()
    }

    fn clear_colour(&self) -> Colour {
        self.themes[self.active].clear_colour()
    }
}

impl<Draw> ThemeApi for MultiTheme<Draw> {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        // Slightly inefficient, but sufficient: update both
        // (Otherwise we would have to call set_colours in set_theme too.)
        let mut action = ThemeAction::None;
        for theme in &mut self.themes {
            action = action.max(theme.set_font_size(size));
        }
        action
    }

    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        // Slightly inefficient, but sufficient: update all
        // (Otherwise we would have to call set_colours in set_theme too.)
        let mut action = ThemeAction::None;
        for theme in &mut self.themes {
            action = action.max(theme.set_colours(scheme));
        }
        action
    }

    fn set_theme(&mut self, theme: &str) -> ThemeAction {
        if let Some(index) = self.names.get(theme).cloned() {
            if index != self.active {
                self.active = index;
                return ThemeAction::ThemeResize;
            }
        }
        ThemeAction::None
    }
}
