// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::any::Any;
use std::f32;

use kas::draw::{self, TextClass};
use kas::geom::{Size, Vec2};
use kas::layout::{AxisInfo, Margins, SizeRules, StretchPolicy};
use kas::text::{TextApi, TextApiExt};

/// Parameterisation of [`Dimensions`]
///
/// All dimensions are multiplied by the DPI factor, then rounded to the
/// nearest integer. Example: `(2.0 * 1.25).round() = 3.0`.
#[derive(Clone, Debug)]
pub struct DimensionsParams {
    /// Space between elements
    pub outer_margin: f32,
    /// Margin inside a frame before contents
    pub inner_margin: f32,
    /// Frame size
    pub frame_size: f32,
    /// Button frame size (non-flat outer region)
    pub button_frame: f32,
    /// Scrollbar minimum handle size
    pub scrollbar_size: Vec2,
    /// Slider minimum handle size
    pub slider_size: Vec2,
    /// Progress bar size (horizontal)
    pub progress_bar: Vec2,
}

/// Dimensions available within [`DimensionsWindow`]
#[derive(Clone, Debug)]
pub struct Dimensions {
    pub scale_factor: f32,
    pub dpp: f32,
    pub pt_size: f32,
    pub font_marker_width: f32,
    pub line_height: i32,
    pub min_line_length: i32,
    pub ideal_line_length: i32,
    pub outer_margin: u16,
    pub inner_margin: u16,
    pub frame: i32,
    pub button_frame: i32,
    pub checkbox: i32,
    pub scrollbar: Size,
    pub slider: Size,
    pub progress_bar: Size,
}

impl Dimensions {
    pub fn new(params: DimensionsParams, pt_size: f32, scale_factor: f32) -> Self {
        let font_id = Default::default();
        let dpp = scale_factor * (96.0 / 72.0);
        let dpem = dpp * pt_size;
        let line_height = kas::text::fonts::fonts().get(font_id).height(dpem).ceil() as i32;

        let outer_margin = (params.outer_margin * scale_factor).round() as u16;
        let inner_margin = (params.inner_margin * scale_factor).round() as u16;
        let frame = (params.frame_size * scale_factor).round() as i32;
        Dimensions {
            scale_factor,
            dpp,
            pt_size,
            font_marker_width: (1.6 * scale_factor).round().max(1.0),
            line_height,
            min_line_length: (8.0 * dpem).round() as i32,
            ideal_line_length: (24.0 * dpem).round() as i32,
            outer_margin,
            inner_margin,
            frame,
            button_frame: (params.button_frame * scale_factor).round() as i32,
            checkbox: (9.0 * dpp).round() as i32 + 2 * (i32::from(inner_margin) + frame),
            scrollbar: Size::from(params.scrollbar_size * scale_factor),
            slider: Size::from(params.slider_size * scale_factor),
            progress_bar: Size::from(params.progress_bar * scale_factor),
        }
    }
}

/// A convenient implementation of [`crate::Window`]
pub struct DimensionsWindow {
    pub dims: Dimensions,
}

impl DimensionsWindow {
    pub fn new(dims: DimensionsParams, pt_size: f32, scale_factor: f32) -> Self {
        DimensionsWindow {
            dims: Dimensions::new(dims, pt_size, scale_factor),
        }
    }
}

impl crate::Window for DimensionsWindow {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = SizeHandle<'static>;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = SizeHandle<'a>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle<'a>(&'a mut self) -> Self::SizeHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        let h: SizeHandle<'a> = SizeHandle::new(&self.dims);
        std::mem::transmute(h)
    }
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self) -> Self::SizeHandle<'a> {
        SizeHandle::new(&self.dims)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct SizeHandle<'a> {
    dims: &'a Dimensions,
}

impl<'a> SizeHandle<'a> {
    pub fn new(dims: &'a Dimensions) -> Self {
        SizeHandle { dims }
    }
}

impl<'a> draw::SizeHandle for SizeHandle<'a> {
    fn scale_factor(&self) -> f32 {
        self.dims.scale_factor
    }

    fn frame(&self) -> Size {
        let f = self.dims.frame;
        Size::splat(f)
    }
    fn menu_frame(&self) -> Size {
        let f = self.dims.frame;
        Size::new(f, f / 2)
    }

    fn inner_margin(&self) -> Size {
        Size::splat(self.dims.inner_margin.into())
    }

    fn outer_margins(&self) -> Margins {
        Margins::splat(self.dims.outer_margin)
    }

    fn line_height(&self, _: TextClass) -> i32 {
        self.dims.line_height
    }

    fn text_bound(
        &mut self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        let required = text.update_env(|env| {
            env.set_dpp(self.dims.dpp);
            env.set_pt_size(self.dims.pt_size);

            let mut bounds = kas::text::Vec2::INFINITY;
            if let Some(size) = axis.size_other_if_fixed(false) {
                bounds.1 = size as f32;
            } else if let Some(size) = axis.size_other_if_fixed(true) {
                bounds.0 = size as f32;
            }
            env.set_bounds(bounds);

            env.set_wrap(match class {
                TextClass::Label | TextClass::EditMulti => true,
                _ => false,
            });
        });

        let margin = match class {
            TextClass::Label | TextClass::LabelSingle => self.dims.outer_margin,
            TextClass::Button | TextClass::Edit | TextClass::EditMulti => self.dims.inner_margin,
        };
        let margins = (margin, margin);
        if axis.is_horizontal() {
            let bound = (required.0).ceil() as i32;
            let min = self.dims.min_line_length;
            let ideal = self.dims.ideal_line_length;
            // NOTE: using different variable-width stretch policies here can
            // cause problems (e.g. edit boxes greedily consuming too much
            // space). This is a hard layout problem; for now don't do this.
            let (min, ideal, policy) = match class {
                TextClass::Edit | TextClass::EditMulti => (min, ideal, StretchPolicy::LowUtility),
                _ => (bound.min(min), bound.min(ideal), StretchPolicy::LowUtility),
            };
            SizeRules::new(min, ideal, margins, policy)
        } else {
            let min = match class {
                TextClass::Label => (required.1).ceil() as i32,
                TextClass::LabelSingle | TextClass::Button | TextClass::Edit => {
                    self.dims.line_height
                }
                TextClass::EditMulti => self.dims.line_height * 3,
            };
            let ideal = ((required.1).ceil() as i32).max(min);
            let stretch = match class {
                TextClass::Button | TextClass::Edit | TextClass::LabelSingle => {
                    StretchPolicy::Fixed
                }
                TextClass::EditMulti => StretchPolicy::HighUtility,
                _ => StretchPolicy::Filler,
            };
            SizeRules::new(min, ideal, margins, stretch)
        }
    }

    fn edit_marker_width(&self) -> f32 {
        self.dims.font_marker_width
    }

    fn button_surround(&self) -> (Size, Size) {
        let s = Size::splat(self.dims.button_frame);
        (s, s)
    }

    fn edit_surround(&self) -> (Size, Size) {
        let s = Size::splat(self.dims.frame + i32::from(self.dims.inner_margin));
        (s, s)
    }

    fn checkbox(&self) -> Size {
        Size::splat(self.dims.checkbox)
    }

    #[inline]
    fn radiobox(&self) -> Size {
        self.checkbox()
    }

    fn scrollbar(&self) -> (Size, i32) {
        let size = self.dims.scrollbar;
        (size, 2 * size.0)
    }

    fn slider(&self) -> (Size, i32) {
        let size = self.dims.slider;
        (size, 2 * size.0)
    }

    fn progress_bar(&self) -> Size {
        self.dims.progress_bar
    }
}
