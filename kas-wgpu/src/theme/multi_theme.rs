// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use std::collections::HashMap;
use std::marker::Unsize;
use wgpu_glyph::Font;

use kas::draw::Colour;
use kas::geom::Rect;
use kas::theme::{self, StackDST, Theme, ThemeAction, ThemeApi};

use super::DimensionsWindow;
use crate::draw::DrawPipe;

/// Supported theme type
type DynTheme = dyn Theme<DrawPipe, Window = DimensionsWindow>;

/// Wrapper around mutliple themes, supporting run-time switching
pub struct MultiTheme {
    names: HashMap<String, usize>,
    themes: Vec<StackDST<DynTheme>>,
    active: usize,
}

pub struct MultiThemeBuilder {
    names: HashMap<String, usize>,
    themes: Vec<StackDST<DynTheme>>,
}

impl MultiTheme {
    /// Construct with builder pattern
    pub fn builder() -> MultiThemeBuilder {
        MultiThemeBuilder {
            names: HashMap::new(),
            themes: vec![],
        }
    }
}

impl MultiThemeBuilder {
    /// Add a theme
    pub fn add<S: ToString, U>(mut self, name: S, theme: U) -> Self
    where
        U: Unsize<DynTheme>,
        Box<U>: Unsize<DynTheme>,
    {
        let index = self.themes.len();
        self.names.insert(name.to_string(), index);
        self.themes.push(StackDST::new_or_boxed(theme));
        self
    }

    /// Build
    ///
    /// Fails if no themes were added.
    pub fn try_build(self) -> Result<MultiTheme, ()> {
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
    pub fn build(self) -> MultiTheme {
        self.try_build()
            .unwrap_or_else(|_| panic!("MultiThemeBuilder: no themes added"))
    }
}

impl Theme<DrawPipe> for MultiTheme {
    type Window = DimensionsWindow;

    fn new_window(&self, draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        self.themes[self.active].new_window(draw, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        self.themes[self.active].update_window(window, dpi_factor)
    }

    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut DrawPipe,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> StackDST<dyn theme::DrawHandle> {
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

impl ThemeApi for MultiTheme {
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
