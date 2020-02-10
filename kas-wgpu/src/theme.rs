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
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules, StretchPolicy};
use kas::theme::{self, TextClass, TextProperties};
use kas::Align;
use kas::Direction::{self, Horizontal, Vertical};

use crate::draw::{DrawPipe, DrawShaded, DrawText, ShadeStyle, Vec2};
use crate::resources::colours::ThemeColours;

/// A simple, inflexible theme providing a sample implementation.
#[derive(Clone, Debug)]
pub struct SampleTheme {
    font_size: f32,
    cols: ThemeColours,
}

impl SampleTheme {
    /// Construct
    pub fn new() -> Self {
        SampleTheme {
            font_size: 18.0,
            cols: ThemeColours::new(),
        }
    }

    /// Set font size. Default is 18.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }
}

#[doc(hidden)]
pub struct SampleWindow {
    font_size: f32, // unscaled by DPI
    dims: ThemeDimensions,
}

#[derive(Clone, Debug)]
struct ThemeDimensions {
    font_scale: f32,
    line_height: u32,
    min_line_length: u32,
    max_line_length: u32,
    margin: u32,
    frame: u32,
    button_frame: u32,
    checkbox: u32,
    scrollbar: u32,
}

/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;
/// Frame size (adjusted as above)
const FRAME_SIZE: f32 = 5.0;
/// Button frame size (non-flat outer region)
const BUTTON_FRAME: f32 = 5.0;
/// Scrollbar width & min length
const SCROLLBAR_SIZE: f32 = 8.0;

impl SampleWindow {
    fn new(font_size: f32, dpi_factor: f32) -> Self {
        SampleWindow {
            font_size,
            dims: ThemeDimensions::new(font_size, dpi_factor),
        }
    }
}

impl ThemeDimensions {
    fn new(font_size: f32, dpi_factor: f32) -> Self {
        let font_scale = font_size * dpi_factor;
        let line_height = font_scale.round() as u32;
        let margin = (MARGIN * dpi_factor).round() as u32;
        let frame = (FRAME_SIZE * dpi_factor).round() as u32;
        ThemeDimensions {
            font_scale,
            line_height,
            min_line_length: line_height * 10,
            max_line_length: line_height * 40,
            margin,
            frame,
            button_frame: (BUTTON_FRAME * dpi_factor).round() as u32,
            checkbox: (font_scale * 0.7).round() as u32 + 2 * (margin + frame),
            scrollbar: (SCROLLBAR_SIZE * dpi_factor).round() as u32,
        }
    }
}

#[doc(hidden)]
pub struct SizeHandle<'a> {
    draw: &'a mut DrawPipe,
    window: &'a mut SampleWindow,
}

impl theme::Window<DrawPipe> for SampleWindow {
    type SizeHandle = SizeHandle<'static>;

    unsafe fn size_handle<'a>(&'a mut self, draw: &'a mut DrawPipe) -> Self::SizeHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        SizeHandle {
            draw: transmute::<&'a mut DrawPipe, &'static mut DrawPipe>(draw),
            window: transmute::<&'a mut Self, &'static mut Self>(self),
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_dpi_factor(&mut self, dpi_factor: f32) {
        self.dims = ThemeDimensions::new(self.font_size, dpi_factor)
    }
}

impl<'a> theme::SizeHandle for SizeHandle<'a> {
    fn outer_frame(&self) -> (Size, Size) {
        let f = self.window.dims.frame as u32;
        (Size::uniform(f), Size::uniform(f))
    }

    fn inner_margin(&self) -> Size {
        Size::uniform(self.window.dims.margin as u32)
    }

    fn outer_margin(&self) -> Size {
        Size::uniform(self.window.dims.margin as u32)
    }

    fn line_height(&self, _: TextClass) -> u32 {
        self.window.dims.line_height
    }

    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules {
        let font_scale = self.window.dims.font_scale;
        let line_height = self.window.dims.line_height;
        let draw = &mut self.draw;
        let mut bound = |dir: Direction| -> u32 {
            let layout = match class {
                TextClass::Label | TextClass::EditMulti => Layout::default_wrap(),
                TextClass::Button | TextClass::Edit => Layout::default_single_line(),
            };
            let mut bounds = (f32::INFINITY, f32::INFINITY);
            if let Some(size) = axis.size_other_if_fixed(Horizontal) {
                bounds.1 = size as f32;
            } else if let Some(size) = axis.size_other_if_fixed(Vertical) {
                bounds.0 = size as f32;
            }

            let bounds = draw.glyph_bounds(Section {
                text,
                screen_position: (0.0, 0.0),
                scale: Scale::uniform(font_scale),
                bounds,
                layout,
                ..Section::default()
            });

            bounds
                .map(|(min, max)| match dir {
                    Horizontal => (max - min).0,
                    Vertical => (max - min).1,
                } as u32)
                .unwrap_or(0)
        };

        let inner = if axis.is_horizontal() {
            let bound = bound(Horizontal);
            let min = match class {
                TextClass::Edit | TextClass::EditMulti => self.window.dims.min_line_length,
                _ => bound.min(self.window.dims.min_line_length),
            };
            let ideal = bound.min(self.window.dims.max_line_length);
            SizeRules::new(min, ideal, StretchPolicy::LowUtility)
        } else
        /* vertical */
        {
            let min = match class {
                TextClass::EditMulti => line_height * 3,
                _ => line_height,
            };
            let ideal = bound(Vertical).max(line_height);
            let stretch = match class {
                TextClass::Button | TextClass::Edit => StretchPolicy::Fixed,
                _ => StretchPolicy::Filler,
            };
            SizeRules::new(min, ideal, stretch)
        };
        let margin = SizeRules::fixed(2 * self.window.dims.margin as u32);
        inner + margin
    }

    fn button_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.window.dims.button_frame);
        (s, s)
    }

    fn edit_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.window.dims.frame as u32);
        (s, s)
    }

    fn checkbox(&self) -> Size {
        Size::uniform(self.window.dims.checkbox)
    }

    #[inline]
    fn radiobox(&self) -> Size {
        self.checkbox()
    }

    fn scrollbar(&self) -> (u32, u32, u32) {
        let s = self.window.dims.scrollbar as u32;
        (s, s, 2 * s)
    }
}

#[doc(hidden)]
pub struct DrawHandle<'a> {
    draw: &'a mut DrawPipe,
    window: &'a mut SampleWindow,
    cols: &'a ThemeColours,
    rect: Rect,
    offset: Coord,
    pass: usize,
}

impl theme::Theme<DrawPipe> for SampleTheme {
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
        let style = ShadeStyle::Round(Vec2(0.6, -0.6));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, self.cols.frame);
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
        let style = ShadeStyle::Round(Vec2(0.0, 0.6));
        self.draw.shaded_frame(self.pass, outer, inner, style, col);

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
        let style = ShadeStyle::Square(Vec2(0.0, -0.8));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, self.cols.frame);

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
        let style = ShadeStyle::Square(Vec2(0.0, -0.8));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, self.cols.frame);

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
        rect: Rect,
        dir: Direction,
        h_len: u32,
        h_pos: u32,
        highlights: HighlightState,
    ) {
        let mut outer = rect + self.offset;

        // TODO: also draw slider behind handle: needs an extra layer?

        let half_width = if dir == Horizontal {
            outer.pos.0 += h_pos as i32;
            outer.size.0 = h_len;
            outer.size.1 / 2
        } else {
            outer.pos.1 += h_pos as i32;
            outer.size.1 = h_len;
            outer.size.0 / 2
        };

        let inner = outer.shrink(half_width);
        let style = ShadeStyle::Round(Vec2(0.0, 0.6));
        let col = self.cols.scrollbar_state(highlights);
        self.draw.shaded_frame(self.pass, outer, inner, style, col);
        self.draw.rect(self.pass, inner, col);
    }
}
