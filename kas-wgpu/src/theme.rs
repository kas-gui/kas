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
use kas::layout::{AxisInfo, SizeRules};
use kas::theme::{self, Align, TextClass, TextProperties};

use crate::draw::{DrawPipe, DrawShaded, DrawText, ShadeStyle, Vec2};

/// A simple, inflexible theme providing a sample implementation.
#[derive(Copy, Clone, Debug, Default)]
pub struct SampleTheme {
    font_size: f32,
}

impl SampleTheme {
    /// Construct
    pub fn new() -> Self {
        SampleTheme { font_size: 18.0 }
    }

    /// Set font size. Default is 18.
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size;
    }
}

#[doc(hidden)]
pub struct SampleWindow {
    font_size: f32,
    font_scale: u32,
    margin: u32,
    frame_size: u32,
    button_frame: u32,
    scrollbar_size: u32,
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

/// Background colour
pub const BACKGROUND: Colour = Colour::grey(0.7);
/// Frame colour
pub const FRAME: Colour = BACKGROUND;
/// Text background
pub const TEXT_AREA: Colour = Colour::grey(1.0);

/// Text in text area
pub const TEXT: Colour = Colour::grey(0.0);
/// Text on background
pub const LABEL_TEXT: Colour = Colour::grey(0.0);
/// Text on button
pub const BUTTON_TEXT: Colour = Colour::grey(1.0);

fn nav_colour(highlights: HighlightState) -> Option<Colour> {
    if highlights.key_focus {
        Some(Colour::new(1.0, 0.7, 0.5))
    } else {
        None
    }
}

fn button_colour(highlights: HighlightState, show: bool) -> Option<Colour> {
    if highlights.depress {
        Some(Colour::new(0.15, 0.525, 0.75))
    } else if show && highlights.hover {
        Some(Colour::new(0.25, 0.8, 1.0))
    } else if show {
        Some(Colour::new(0.2, 0.7, 1.0))
    } else {
        None
    }
}

impl SampleWindow {
    fn new(font_size: f32, dpi_factor: f32) -> Self {
        SampleWindow {
            font_size,
            font_scale: (font_size * dpi_factor).round() as u32,
            margin: (MARGIN * dpi_factor).round() as u32,
            frame_size: (FRAME_SIZE * dpi_factor).round() as u32,
            button_frame: (BUTTON_FRAME * dpi_factor).round() as u32,
            scrollbar_size: (SCROLLBAR_SIZE * dpi_factor).round() as u32,
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

    fn set_dpi_factor(&mut self, factor: f32) {
        *self = SampleWindow::new(self.font_size, factor)
    }
}

impl<'a> theme::SizeHandle for SizeHandle<'a> {
    fn outer_frame(&self) -> (Size, Size) {
        let f = self.window.frame_size as u32;
        (Size::uniform(f), Size::uniform(f))
    }

    fn inner_margin(&self) -> Size {
        Size::uniform(self.window.margin as u32)
    }

    fn outer_margin(&self) -> Size {
        Size::uniform(self.window.margin as u32)
    }

    fn line_height(&self, _: TextClass) -> u32 {
        self.window.font_scale
    }

    fn text_bound(
        &mut self,
        text: &str,
        _: TextClass,
        multi_line: bool,
        axis: AxisInfo,
    ) -> SizeRules {
        let font_scale = self.window.font_scale;
        let line_height = font_scale;
        let draw = &mut self.draw;
        let mut bound = |vert: bool| -> u32 {
            let layout = match multi_line {
                false => Layout::default_single_line(),
                true => Layout::default_wrap(),
            };
            let mut bounds = (f32::INFINITY, f32::INFINITY);
            if let Some(size) = axis.fixed(false) {
                bounds.1 = size as f32;
            } else if let Some(size) = axis.fixed(true) {
                bounds.0 = size as f32;
            }

            let bounds = draw.glyph_bounds(Section {
                text,
                screen_position: (0.0, 0.0),
                scale: Scale::uniform(font_scale as f32),
                bounds,
                layout,
                ..Section::default()
            });

            bounds
                .map(|(min, max)| match vert {
                    false => (max - min).0,
                    true => (max - min).1,
                } as u32)
                .unwrap_or(0)
        };

        let inner = if !axis.vertical() {
            let min = 3 * line_height;
            SizeRules::variable(min, bound(false).max(min))
        } else {
            SizeRules::variable(line_height, bound(true).max(line_height))
        };
        let margin = SizeRules::fixed(2 * self.window.margin as u32);
        inner + margin
    }

    fn button_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.window.button_frame);
        (s, s)
    }

    fn edit_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.window.frame_size as u32);
        (s, s)
    }

    fn checkbox(&self) -> Size {
        Size::uniform(2 * (self.window.frame_size + self.window.margin) + self.window.font_scale)
    }

    fn scrollbar(&self) -> (u32, u32, u32) {
        let s = self.window.scrollbar_size as u32;
        (s, s, 2 * s)
    }
}

#[doc(hidden)]
pub struct DrawHandle<'a> {
    draw: &'a mut DrawPipe,
    window: &'a mut SampleWindow,
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
        BACKGROUND
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
        let inner = outer.shrink(self.window.frame_size);
        let style = ShadeStyle::Round(Vec2(0.6, -0.6));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, FRAME);
    }

    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        let outer = rect + self.offset;
        let bounds = Coord::from(rect.size) - Coord::uniform(2 * self.window.margin as i32);

        let col = match props.class {
            TextClass::Label => LABEL_TEXT,
            TextClass::Button => BUTTON_TEXT,
            TextClass::Edit => TEXT,
        };

        // TODO: support justified alignment
        let (h_align, h_offset) = match props.horiz {
            Align::Begin | Align::Justify => (HorizontalAlign::Left, 0),
            Align::Centre => (HorizontalAlign::Center, bounds.0 / 2),
            Align::End => (HorizontalAlign::Right, bounds.0),
        };
        let (v_align, v_offset) = match props.vert {
            Align::Begin | Align::Justify => (VerticalAlign::Top, 0),
            Align::Centre => (VerticalAlign::Center, bounds.1 / 2),
            Align::End => (VerticalAlign::Bottom, bounds.1),
        };

        let text_pos =
            outer.pos + Coord::uniform(self.window.margin as i32) + Coord(h_offset, v_offset);

        let layout = match props.multi_line {
            true => Layout::default_wrap(),
            false => Layout::default_single_line(),
        }
        .h_align(h_align)
        .v_align(v_align);

        self.draw.draw_text(Section {
            text,
            screen_position: Vec2::from(text_pos).into(),
            color: col.into(),
            scale: Scale::uniform(self.window.font_scale as f32),
            bounds: Vec2::from(bounds).into(),
            layout,
            ..Section::default()
        });
    }

    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        let mut outer = rect + self.offset;
        let col = button_colour(highlights, true).unwrap();

        let mut inner = outer.shrink(self.window.button_frame);
        let style = ShadeStyle::Round(Vec2(0.0, 0.6));
        self.draw.shaded_frame(self.pass, outer, inner, style, col);

        if highlights.key_focus {
            outer = inner;
            inner = outer.shrink(self.window.margin);
            let col = nav_colour(highlights).unwrap();
            self.draw.frame(self.pass, outer, inner, col);
        }

        self.draw.rect(self.pass, inner, col);
    }

    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        let mut outer = rect + self.offset;

        let mut inner = outer.shrink(self.window.frame_size);
        let style = ShadeStyle::Square(Vec2(0.0, -0.8));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, FRAME);

        if highlights.key_focus {
            outer = inner;
            inner = outer.shrink(self.window.margin);
            let col = nav_colour(highlights).unwrap();
            self.draw.frame(self.pass, outer, inner, col);
        }

        self.draw.rect(self.pass, inner, TEXT_AREA);
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        let mut outer = rect + self.offset;

        // TODO: remove this hack when the layout engine can align instead of stretch
        let pref_size = Size::uniform(
            2 * (self.window.frame_size + self.window.margin) + self.window.font_scale,
        );
        if outer.size.0 > pref_size.0 {
            outer.size.0 = pref_size.0;
        }
        if outer.size.1 > pref_size.1 {
            outer.pos.1 += ((outer.size.1 - pref_size.1) / 2) as i32;
            outer.size.1 = pref_size.1;
        }

        let mut inner = outer.shrink(self.window.frame_size);
        let style = ShadeStyle::Square(Vec2(0.0, -0.8));
        self.draw
            .shaded_frame(self.pass, outer, inner, style, FRAME);

        if checked || highlights.any() {
            outer = inner;
            inner = outer.shrink(self.window.margin);
            let col = nav_colour(highlights).unwrap_or(TEXT_AREA);
            self.draw.frame(self.pass, outer, inner, col);
        }

        let col = button_colour(highlights, checked).unwrap_or(TEXT_AREA);
        self.draw.rect(self.pass, inner, col);
    }

    fn scrollbar(
        &mut self,
        rect: Rect,
        dir: bool,
        h_len: u32,
        h_pos: u32,
        highlights: HighlightState,
    ) {
        let mut outer = rect + self.offset;

        // TODO: also draw slider behind handle: needs an extra layer?

        let half_width = if !dir {
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
        let col = button_colour(highlights, true).unwrap();
        self.draw.shaded_frame(self.pass, outer, inner, style, col);
        self.draw.rect(self.pass, inner, col);
    }
}
