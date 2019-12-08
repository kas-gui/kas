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

use kas::class::{Align, Class};
use kas::draw::*;
use kas::event::{HighlightState, Manager};
use kas::geom::{Coord, Rect, Size};
use kas::layout::{AxisInfo, SizeRules};
use kas::theme::{self, TextClass, TextProperties};
use kas::Widget;

use crate::draw::*;

/// A simple, inflexible theme providing a sample implementation.
#[derive(Copy, Clone, Debug, Default)]
pub struct SampleTheme;

impl SampleTheme {
    /// Construct
    pub fn new() -> Self {
        SampleTheme
    }
}

#[doc(hidden)]
pub struct SampleWindow {
    font_scale: f32,
    margin: f32,
    frame_size: f32,
    button_frame: f32,
}

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;
/// Frame size (adjusted as above)
const FRAME_SIZE: f32 = 5.0;
/// Button frame size (non-flat outer region)
const BUTTON_FRAME: f32 = 5.0;

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
        Some(Colour::new(0.2, 0.6, 0.8))
    } else if show && highlights.hover {
        Some(Colour::new(0.25, 0.8, 1.0))
    } else if show {
        Some(Colour::new(0.2, 0.7, 1.0))
    } else {
        None
    }
}

impl SampleWindow {
    fn new(dpi_factor: f32) -> Self {
        SampleWindow {
            font_scale: (FONT_SIZE * dpi_factor).round(),
            margin: (MARGIN * dpi_factor).round(),
            frame_size: (FRAME_SIZE * dpi_factor).round(),
            button_frame: (BUTTON_FRAME * dpi_factor).round(),
        }
    }
}

// This theme does not need shared resources, hence we can use the same type for
// theme::Theme::DrawHandle and theme::Window::SizeHandle.
#[doc(hidden)]
pub struct SampleHandle<'a> {
    draw: &'a mut DrawPipe,
    window: &'a mut SampleWindow,
}

impl theme::Window<DrawPipe> for SampleWindow {
    type SizeHandle = SampleHandle<'static>;

    unsafe fn size_handle<'a>(&'a mut self, draw: &'a mut DrawPipe) -> Self::SizeHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        SampleHandle {
            draw: transmute::<&'a mut DrawPipe, &'static mut DrawPipe>(draw),
            window: transmute::<&'a mut Self, &'static mut Self>(self),
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_dpi_factor(&mut self, factor: f32) {
        *self = SampleWindow::new(factor)
    }
}

impl<'a> theme::SizeHandle for SampleHandle<'a> {
    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        let font_scale = self.window.font_scale;
        let line_height = font_scale as u32;
        let draw = &mut self.draw;
        let mut bound = |vert: bool| -> u32 {
            let layout = match widget.class() {
                Class::Entry(_) => Layout::default_single_line(),
                _ => Layout::default_wrap(),
            };
            let bounds = widget.class().text().and_then(|text| {
                let mut bounds = (f32::INFINITY, f32::INFINITY);
                if let Some(size) = axis.fixed(false) {
                    bounds.1 = size as f32;
                } else if let Some(size) = axis.fixed(true) {
                    bounds.0 = size as f32;
                }
                draw.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(font_scale),
                    bounds,
                    layout,
                    ..Section::default()
                })
            });

            bounds
                .map(|(min, max)| match vert {
                    false => (max - min).0,
                    true => (max - min).1,
                } as u32)
                .unwrap_or(0)
        };

        let inner = match widget.class() {
            Class::Container | Class::None => return SizeRules::EMPTY,
            Class::Entry(_) => {
                let frame = 2 * self.window.frame_size as u32;
                if !axis.vertical() {
                    let min = frame + 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    let min = frame + line_height;
                    SizeRules::variable(min, bound(true).max(min))
                }
            }
        };
        let margin = SizeRules::fixed(2 * self.window.margin as u32);
        inner + margin
    }

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
        self.window.font_scale as u32
    }

    fn text_bound(
        &mut self,
        text: &str,
        _: TextClass,
        multi_line: bool,
        axis: AxisInfo,
    ) -> SizeRules {
        let font_scale = self.window.font_scale;
        let line_height = font_scale as u32;
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
                scale: Scale::uniform(font_scale),
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
        let s = Size::uniform(self.window.button_frame as u32);
        (s, s)
    }

    fn checkbox(&self) -> Size {
        Size::uniform(
            (2.0 * (self.window.frame_size + self.window.margin) + self.window.font_scale) as u32,
        )
    }
}

impl theme::Theme<DrawPipe> for SampleTheme {
    type Window = SampleWindow;
    type DrawHandle = SampleHandle<'static>;

    /// Construct per-window storage
    ///
    /// See also documentation on [`ThemeWindow::set_dpi_factor`].
    fn new_window(&self, _draw: &mut DrawPipe, dpi_factor: f32) -> Self::Window {
        SampleWindow::new(dpi_factor)
    }

    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut DrawPipe,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        SampleHandle {
            draw: transmute::<&'a mut DrawPipe, &'static mut DrawPipe>(draw),
            window: transmute::<&'a mut Self::Window, &'static mut Self::Window>(window),
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

impl<'a> theme::DrawHandle for SampleHandle<'a> {
    fn draw(&mut self, ev_mgr: &Manager, widget: &dyn kas::Widget) {
        // This is a hacky draw routine just to show where widgets are.
        let w_id = widget.id();
        let highlights = ev_mgr.highlight_state(w_id);

        // Note: coordinates place the origin at the top-left.
        let rect = widget.rect();
        let p = Vec2::from(rect.pos);
        let size = Vec2::from(rect.size);
        let mut quad = Quad(p, p + size);

        let background;

        let margin = self.window.margin;
        let scale = Scale::uniform(self.window.font_scale);
        let mut bounds = size - 2.0 * margin;

        let f = self.window.frame_size;

        let mut _string; // Entry needs this to give a valid lifetime
        let text: Option<(&str, Colour)>;
        let layout;

        match widget.class() {
            Class::Container | Class::None => {
                // do not draw containers
                return;
            }
            Class::Entry(cls) => {
                let outer = quad;
                quad.shrink(f);
                let style = Style::Square(Vec2(0.0, -0.8));
                self.draw.draw_frame(outer, quad, style, FRAME);
                bounds = bounds - 2.0 * f;
                layout = Layout::default_single_line();

                background = Some(TEXT_AREA);

                _string = cls.get_text().to_string();
                if ev_mgr.char_focus(w_id) {
                    // TODO: proper edit character and positioning
                    _string.push('|');
                }
                text = Some((&_string, TEXT));
            }
        }

        if let Some((text, colour)) = text {
            let alignments = widget.class().alignments();
            // TODO: support justified alignment
            let (h_align, h_offset) = match alignments.0 {
                Align::Begin | Align::Justify => (HorizontalAlign::Left, 0.0),
                Align::Center => (HorizontalAlign::Center, 0.5 * bounds.0),
                Align::End => (HorizontalAlign::Right, bounds.0),
            };
            let (v_align, v_offset) = match alignments.1 {
                Align::Begin | Align::Justify => (VerticalAlign::Top, 0.0),
                Align::Center => (VerticalAlign::Center, 0.5 * bounds.1),
                Align::End => (VerticalAlign::Bottom, bounds.1),
            };
            let layout = layout.h_align(h_align).v_align(v_align);
            let text_pos = quad.0 + margin + Vec2(h_offset, v_offset);

            self.draw.draw_text(Section {
                text,
                screen_position: text_pos.into(),
                color: colour.into(),
                scale,
                bounds: bounds.into(),
                layout,
                ..Section::default()
            });
        }

        if highlights.any() {
            let outer = quad;
            quad.shrink(margin);
            let style = Style::Flat;
            let col = nav_colour(highlights).unwrap_or(TEXT_AREA);
            self.draw.draw_frame(outer, quad, style, col);
        }

        if let Some(background) = background {
            self.draw.draw_quad(quad, Style::Flat, background.into());
        }
    }

    fn outer_frame(&mut self, rect: Rect) {
        let p = Vec2::from(rect.pos);
        let size = Vec2::from(rect.size);
        let mut quad = Quad(p, p + size);
        let outer = quad;
        quad.shrink(self.window.frame_size);
        let style = Style::Round(Vec2(0.6, -0.6));
        self.draw.draw_frame(outer, quad, style, FRAME);
    }

    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        let pos = Vec2::from(rect.pos);
        let size = Vec2::from(rect.size);
        let quad = Quad(pos, pos + size);
        let bounds = size - 2.0 * self.window.margin;

        let col = match props.class {
            TextClass::Label => LABEL_TEXT,
            TextClass::Button => BUTTON_TEXT,
        };

        // TODO: support justified alignment
        let (h_align, h_offset) = match props.horiz {
            Align::Begin | Align::Justify => (HorizontalAlign::Left, 0.0),
            Align::Center => (HorizontalAlign::Center, 0.5 * bounds.0),
            Align::End => (HorizontalAlign::Right, bounds.0),
        };
        let (v_align, v_offset) = match props.vert {
            Align::Begin | Align::Justify => (VerticalAlign::Top, 0.0),
            Align::Center => (VerticalAlign::Center, 0.5 * bounds.1),
            Align::End => (VerticalAlign::Bottom, bounds.1),
        };

        let text_pos = quad.0 + self.window.margin + Vec2(h_offset, v_offset);

        let layout = match props.multi_line {
            true => Layout::default_wrap(),
            false => Layout::default_single_line(),
        }
        .h_align(h_align)
        .v_align(v_align);

        self.draw.draw_text(Section {
            text,
            screen_position: text_pos.into(),
            color: col.into(),
            scale: Scale::uniform(self.window.font_scale),
            bounds: bounds.into(),
            layout,
            ..Section::default()
        });
    }

    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        let pos = Vec2::from(rect.pos);
        let size = Vec2::from(rect.size);
        let mut quad = Quad(pos, pos + size);

        let col = button_colour(highlights, true).unwrap();

        let f = self.window.button_frame;
        let outer = quad;
        quad.shrink(f);
        let style = Style::Round(Vec2(0.0, 0.6));
        self.draw.draw_frame(outer, quad, style, col);

        if highlights.key_focus {
            let outer = quad;
            quad.shrink(self.window.margin);
            let col = nav_colour(highlights).unwrap();
            self.draw.draw_frame(outer, quad, Style::Flat, col);
        }

        self.draw.draw_quad(quad, Style::Flat, col);
    }

    fn checkbox(&mut self, pos: Coord, checked: bool, highlights: HighlightState) {
        let pos = Vec2::from(pos);
        let size = 2.0 * (self.window.frame_size + self.window.margin) + self.window.font_scale;
        let size = Vec2::splat(size);
        let mut quad = Quad(pos, pos + size);

        let outer = quad;
        quad.shrink(self.window.frame_size);
        let style = Style::Square(Vec2(0.0, -0.8));
        self.draw.draw_frame(outer, quad, style, FRAME);

        if checked || highlights.any() {
            let outer = quad;
            quad.shrink(self.window.margin);
            let col = nav_colour(highlights).unwrap_or(TEXT_AREA);
            self.draw.draw_frame(outer, quad, Style::Flat, col);
        }

        let col = button_colour(highlights, checked).unwrap_or(TEXT_AREA);
        self.draw.draw_quad(quad, Style::Flat, col);
    }
}
