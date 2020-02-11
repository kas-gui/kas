// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::any::Any;
use std::f32;
use wgpu_glyph::{Font, HorizontalAlign, Layout, Scale, Section, VerticalAlign};

use kas::draw::{Colour, Draw};
use kas::event::HighlightState;
use kas::geom::{Coord, Rect};
use kas::theme::{self, TextClass, TextProperties};
use kas::Align;
use kas::Direction;

use super::{Dimensions, DimensionsParams, SizeHandle};
use crate::draw::{DrawPipe, DrawText, Vec2};
use crate::resources::colours::ThemeColours;

/// A simple flat theme.
#[derive(Clone, Debug)]
pub struct FlatTheme {
    font_size: f32,
    cols: ThemeColours,
}

impl FlatTheme {
    /// Construct
    pub fn new() -> Self {
        FlatTheme {
            font_size: 18.0,
            cols: ThemeColours::new(),
        }
    }

    /// Set font size. Default is 18.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }
}

pub struct SampleWindow {
    font_size: f32, // unscaled by DPI
    dims: Dimensions,
}

const DIMS: DimensionsParams = DimensionsParams {
    margin: 2.0,
    frame_size: 5.0,
    button_frame: 3.0,
    scrollbar_size: 8.0,
};

impl SampleWindow {
    fn new(font_size: f32, dpi_factor: f32) -> Self {
        SampleWindow {
            font_size,
            dims: Dimensions::new(DIMS, font_size, dpi_factor),
        }
    }
}

impl theme::Window<DrawPipe> for SampleWindow {
    type SizeHandle = SizeHandle<'static>;

    unsafe fn size_handle<'a>(&'a mut self, draw: &'a mut DrawPipe) -> Self::SizeHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        let handle = SizeHandle::new(draw, &self.dims);
        std::mem::transmute::<SizeHandle<'a>, SizeHandle<'static>>(handle)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_dpi_factor(&mut self, dpi_factor: f32) {
        self.dims = Dimensions::new(DIMS, self.font_size, dpi_factor)
    }
}

pub struct DrawHandle<'a> {
    draw: &'a mut DrawPipe,
    window: &'a mut SampleWindow,
    cols: &'a ThemeColours,
    rect: Rect,
    offset: Coord,
    pass: usize,
}

impl theme::Theme<DrawPipe> for FlatTheme {
    type Window = SampleWindow;
    type DrawHandle = DrawHandle<'static>;

    fn new_window(&self, _draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        SampleWindow::new(self.font_size, dpi_factor)
    }

    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut DrawPipe,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        DrawHandle {
            draw: transmute::<&'a mut DrawPipe, &'static mut DrawPipe>(draw),
            window: transmute::<&'a mut Self::Window, &'static mut Self::Window>(window),
            cols: transmute::<&'a ThemeColours, &'static ThemeColours>(&self.cols),
            rect,
            offset: Coord::ZERO,
            pass: 0,
        }
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        vec![crate::font::get_font()]
    }

    fn light_direction(&self) -> (f32, f32) {
        (0.3, 0.4)
    }

    fn clear_colour(&self) -> Colour {
        self.cols.background
    }

    fn set_colours(&mut self, scheme: &str) -> bool {
        if let Some(scheme) = ThemeColours::open(scheme) {
            self.cols = scheme;
            true
        } else {
            false
        }
    }
}

impl<'a> theme::DrawHandle for DrawHandle<'a> {
    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        f: &mut dyn FnMut(&mut dyn theme::DrawHandle),
    ) {
        let rect = rect + self.offset;
        let pass = self.draw.add_clip_region(rect);
        let mut handle = DrawHandle {
            draw: self.draw,
            window: self.window,
            cols: self.cols,
            rect,
            offset: self.offset - offset,
            pass,
        };
        f(&mut handle);
    }

    fn target_rect(&self) -> Rect {
        // Translate to local coordinates
        self.rect - self.offset
    }

    fn outer_frame(&mut self, rect: Rect) {
        let outer = rect + self.offset;
        let inner = outer.shrink(self.window.dims.frame);
        self.draw.frame(self.pass, outer, inner, self.cols.frame);
    }

    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        let outer = rect + self.offset;
        let bounds = Coord::from(rect.size) - Coord::uniform(2 * self.window.dims.margin as i32);

        let col = match props.class {
            TextClass::Label => self.cols.label_text,
            TextClass::Button => self.cols.button_text,
            TextClass::Edit | TextClass::EditMulti => self.cols.text,
        };

        // TODO: support justified alignment
        let (h_align, h_offset) = match props.horiz {
            Align::Begin | Align::Stretch => (HorizontalAlign::Left, 0),
            Align::Centre => (HorizontalAlign::Center, bounds.0 / 2),
            Align::End => (HorizontalAlign::Right, bounds.0),
        };
        let (v_align, v_offset) = match props.vert {
            Align::Begin | Align::Stretch => (VerticalAlign::Top, 0),
            Align::Centre => (VerticalAlign::Center, bounds.1 / 2),
            Align::End => (VerticalAlign::Bottom, bounds.1),
        };

        let text_pos =
            outer.pos + Coord::uniform(self.window.dims.margin as i32) + Coord(h_offset, v_offset);

        let layout = match props.class {
            TextClass::Label | TextClass::EditMulti => Layout::default_wrap(),
            TextClass::Button | TextClass::Edit => Layout::default_single_line(),
        };
        let layout = layout.h_align(h_align).v_align(v_align);

        self.draw.draw_text(Section {
            text,
            screen_position: Vec2::from(text_pos).into(),
            color: col.into(),
            scale: Scale::uniform(self.window.dims.font_scale),
            bounds: Vec2::from(bounds).into(),
            layout,
            ..Section::default()
        });
    }

    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        let mut outer = rect + self.offset;
        let col = self.cols.button_state(highlights);

        let mut inner = outer.shrink(self.window.dims.button_frame);
        self.draw.frame(self.pass, outer, inner, col);

        if let Some(col) = self.cols.nav_region(highlights) {
            outer = inner;
            inner = outer.shrink(self.window.dims.margin);
            self.draw.frame(self.pass, outer, inner, col);
        }

        self.draw.rect(self.pass, inner, col);
    }

    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        let mut outer = rect + self.offset;

        let mut inner = outer.shrink(self.window.dims.frame);
        self.draw.frame(self.pass, outer, inner, self.cols.frame);

        if let Some(col) = self.cols.nav_region(highlights) {
            outer = inner;
            inner = outer.shrink(self.window.dims.margin);
            self.draw.frame(self.pass, outer, inner, col);
        }

        self.draw.rect(self.pass, inner, self.cols.text_area);
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        let mut outer = rect + self.offset;

        let mut inner = outer.shrink(self.window.dims.frame);
        self.draw.frame(self.pass, outer, inner, self.cols.frame);

        if checked || highlights.any() {
            outer = inner;
            inner = outer.shrink(self.window.dims.margin);
            let col = self
                .cols
                .nav_region(highlights)
                .unwrap_or(self.cols.text_area);
            self.draw.frame(self.pass, outer, inner, col);
        }

        let col = self
            .cols
            .check_mark_state(highlights, checked)
            .unwrap_or(self.cols.text_area);
        self.draw.rect(self.pass, inner, col);
    }

    #[inline]
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        // TODO: distinct
        self.checkbox(rect, checked, highlights);
    }

    fn scrollbar(
        &mut self,
        _rect: Rect,
        h_rect: Rect,
        _dir: Direction,
        highlights: HighlightState,
    ) {
        // TODO: also draw slider behind handle: needs an extra layer?

        let outer = h_rect + self.offset;
        let half_width = outer.size.0.min(outer.size.1) / 2;
        let inner = outer.shrink(half_width);
        let col = self.cols.scrollbar_state(highlights);
        self.draw.frame(self.pass, outer, inner, col);
        self.draw.rect(self.pass, inner, col);
    }
}
