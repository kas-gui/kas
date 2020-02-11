// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::f32;

use wgpu_glyph::{Layout, Scale, Section};

use kas::geom::Size;
use kas::layout::{AxisInfo, SizeRules, StretchPolicy};
use kas::theme::{self, TextClass};
use kas::Direction::{self, Horizontal, Vertical};

use crate::draw::{DrawPipe, DrawText};

/// Parameterisation of [`Dimensions`]
///
/// All dimensions are multiplied by the DPI factor, then rounded to the
/// nearest integer. Example: `(2.0 * 1.25).round() = 3.0`.
#[derive(Clone, Debug)]
pub struct DimensionsParams {
    /// Inner margin
    pub margin: f32,
    /// Frame size
    pub frame_size: f32,
    /// Button frame size (non-flat outer region)
    pub button_frame: f32,
    /// Scrollbar width & min length
    pub scrollbar_size: f32,
}

#[derive(Clone, Debug)]
pub struct Dimensions {
    pub font_scale: f32,
    pub line_height: u32,
    pub min_line_length: u32,
    pub max_line_length: u32,
    pub margin: u32,
    pub frame: u32,
    pub button_frame: u32,
    pub checkbox: u32,
    pub scrollbar: u32,
}

impl Dimensions {
    pub fn new(params: DimensionsParams, font_size: f32, dpi_factor: f32) -> Self {
        let font_scale = font_size * dpi_factor;
        let line_height = font_scale.round() as u32;
        let margin = (params.margin * dpi_factor).round() as u32;
        let frame = (params.frame_size * dpi_factor).round() as u32;
        Dimensions {
            font_scale,
            line_height,
            min_line_length: line_height * 10,
            max_line_length: line_height * 40,
            margin,
            frame,
            button_frame: (params.button_frame * dpi_factor).round() as u32,
            checkbox: (font_scale * 0.7).round() as u32 + 2 * (margin + frame),
            scrollbar: (params.scrollbar_size * dpi_factor).round() as u32,
        }
    }
}

#[doc(hidden)]
pub struct SizeHandle<'a> {
    draw: &'a mut DrawPipe,
    dims: &'a Dimensions,
}

impl<'a> SizeHandle<'a> {
    pub fn new(draw: &'a mut DrawPipe, dims: &'a Dimensions) -> Self {
        SizeHandle { draw, dims }
    }
}

impl<'a> theme::SizeHandle for SizeHandle<'a> {
    fn outer_frame(&self) -> (Size, Size) {
        let f = self.dims.frame as u32;
        (Size::uniform(f), Size::uniform(f))
    }

    fn inner_margin(&self) -> Size {
        Size::uniform(self.dims.margin as u32)
    }

    fn outer_margin(&self) -> Size {
        Size::uniform(self.dims.margin as u32)
    }

    fn line_height(&self, _: TextClass) -> u32 {
        self.dims.line_height
    }

    fn text_bound(&mut self, text: &str, class: TextClass, axis: AxisInfo) -> SizeRules {
        let font_scale = self.dims.font_scale;
        let line_height = self.dims.line_height;
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
                TextClass::Edit | TextClass::EditMulti => self.dims.min_line_length,
                _ => bound.min(self.dims.min_line_length),
            };
            let ideal = bound.min(self.dims.max_line_length);
            SizeRules::new(min, ideal, StretchPolicy::LowUtility)
        } else {
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
        let margin = SizeRules::fixed(2 * self.dims.margin as u32);
        inner + margin
    }

    fn button_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.dims.button_frame);
        (s, s)
    }

    fn edit_surround(&self) -> (Size, Size) {
        let s = Size::uniform(self.dims.frame as u32);
        (s, s)
    }

    fn checkbox(&self) -> Size {
        Size::uniform(self.dims.checkbox)
    }

    #[inline]
    fn radiobox(&self) -> Size {
        self.checkbox()
    }

    fn scrollbar(&self) -> (u32, u32, u32) {
        let s = self.dims.scrollbar as u32;
        (s, s, 2 * s)
    }
}
