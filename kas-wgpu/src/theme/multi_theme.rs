// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Wrapper around mutliple themes, supporting run-time switching

use std::f32;
use wgpu_glyph::Font;

use kas::draw::Colour;
use kas::event::HighlightState;
use kas::geom::{Coord, Rect};
use kas::theme::{self, TextProperties, ThemeAction, ThemeApi};
use kas::Direction;

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

pub enum WhichDrawHandle {
    Flat(<FlatTheme as theme::Theme<DrawPipe>>::DrawHandle),
    Shaded(<ShadedTheme as theme::Theme<DrawPipe>>::DrawHandle),
}

impl theme::Theme<DrawPipe> for MultiTheme {
    type Window = DimensionsWindow;
    type DrawHandle = WhichDrawHandle;

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
    ) -> Self::DrawHandle {
        match self.which {
            WhichTheme::Flat => WhichDrawHandle::Flat(self.flat.draw_handle(draw, window, rect)),
            WhichTheme::Shaded => {
                WhichDrawHandle::Shaded(self.shaded.draw_handle(draw, window, rect))
            }
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

impl theme::DrawHandle for WhichDrawHandle {
    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        f: &mut dyn FnMut(&mut dyn theme::DrawHandle),
    ) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.clip_region(rect, offset, f),
            WhichDrawHandle::Shaded(handle) => handle.clip_region(rect, offset, f),
        }
    }

    fn target_rect(&self) -> Rect {
        match self {
            WhichDrawHandle::Flat(handle) => handle.target_rect(),
            WhichDrawHandle::Shaded(handle) => handle.target_rect(),
        }
    }

    fn outer_frame(&mut self, rect: Rect) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.outer_frame(rect),
            WhichDrawHandle::Shaded(handle) => handle.outer_frame(rect),
        }
    }

    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.text(rect, text, props),
            WhichDrawHandle::Shaded(handle) => handle.text(rect, text, props),
        }
    }

    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.button(rect, highlights),
            WhichDrawHandle::Shaded(handle) => handle.button(rect, highlights),
        }
    }

    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.edit_box(rect, highlights),
            WhichDrawHandle::Shaded(handle) => handle.edit_box(rect, highlights),
        }
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.checkbox(rect, checked, highlights),
            WhichDrawHandle::Shaded(handle) => handle.checkbox(rect, checked, highlights),
        }
    }

    #[inline]
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.radiobox(rect, checked, highlights),
            WhichDrawHandle::Shaded(handle) => handle.radiobox(rect, checked, highlights),
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, dir: Direction, highlights: HighlightState) {
        match self {
            WhichDrawHandle::Flat(handle) => handle.scrollbar(rect, h_rect, dir, highlights),
            WhichDrawHandle::Shaded(handle) => handle.scrollbar(rect, h_rect, dir, highlights),
        }
    }
}
