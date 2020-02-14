// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use std::f32;
use wgpu_glyph::Font;

use kas::draw::Colour;
use kas::geom::Rect;
use kas::theme::{self, StackDST, ThemeAction, ThemeApi};

use super::{DimensionsWindow, FlatTheme, ShadedTheme};
use crate::draw::DrawPipe;

#[derive(Clone, Debug, PartialEq)]
enum WhichTheme {
    Flat,
    Shaded,
}

/// Wrapper around mutliple themes, supporting run-time switching
#[derive(Clone, Debug)]
pub struct MultiTheme {
    which: WhichTheme,
    flat: FlatTheme,
    shaded: ShadedTheme,
}

impl MultiTheme {
    /// Construct
    pub fn new() -> Self {
        MultiTheme {
            which: WhichTheme::Shaded,
            flat: FlatTheme::new(),
            shaded: ShadedTheme::new(),
        }
    }
}

impl theme::Theme<DrawPipe> for MultiTheme {
    type Window = DimensionsWindow;

    fn new_window(&self, draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        match self.which {
            WhichTheme::Flat => self.flat.new_window(draw, dpi_factor),
            WhichTheme::Shaded => self.shaded.new_window(draw, dpi_factor),
        }
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        match self.which {
            WhichTheme::Flat => self.flat.update_window(window, dpi_factor),
            WhichTheme::Shaded => self.shaded.update_window(window, dpi_factor),
        }
    }

    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut DrawPipe,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> StackDST<dyn theme::DrawHandle> {
        match self.which {
            WhichTheme::Flat => self.flat.draw_handle(draw, window, rect),
            WhichTheme::Shaded => self.shaded.draw_handle(draw, window, rect),
        }
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        match self.which {
            WhichTheme::Flat => self.flat.get_fonts(),
            WhichTheme::Shaded => self.shaded.get_fonts(),
        }
    }

    fn light_direction(&self) -> (f32, f32) {
        match self.which {
            WhichTheme::Flat => self.flat.light_direction(),
            WhichTheme::Shaded => self.shaded.light_direction(),
        }
    }

    fn clear_colour(&self) -> Colour {
        match self.which {
            WhichTheme::Flat => self.flat.clear_colour(),
            WhichTheme::Shaded => self.shaded.clear_colour(),
        }
    }
}

impl ThemeApi for MultiTheme {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        // Slightly inefficient, but sufficient: update both
        // (Otherwise we would have to call set_colours in set_theme too.)
        let _ = self.flat.set_font_size(size);
        self.shaded.set_font_size(size)
    }

    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        // Slightly inefficient, but sufficient: update both
        // (Otherwise we would have to call set_colours in set_theme too.)
        let _ = self.flat.set_colours(scheme);
        self.shaded.set_colours(scheme)
    }

    fn set_theme(&mut self, theme: &str) -> ThemeAction {
        match theme {
            "flat" if self.which != WhichTheme::Flat => {
                self.which = WhichTheme::Flat;
                ThemeAction::ThemeResize
            }
            "shaded" if self.which != WhichTheme::Shaded => {
                self.which = WhichTheme::Shaded;
                ThemeAction::ThemeResize
            }
            _ => ThemeAction::None,
        }
    }
}
