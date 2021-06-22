// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use linear_map::LinearMap;
use std::any::Any;
use std::f32;
use std::rc::Rc;

use kas::cast::{Cast, CastFloat, ConvFloat};
use kas::draw::{self, DrawShared, DrawSharedT, DrawableShared, TextClass};
use kas::geom::{Size, Vec2};
use kas::layout::{AxisInfo, FrameRules, Margins, SizeRules, Stretch};
use kas::text::{fonts::FontId, TextApi, TextApiExt};

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
    /// Margin between text elements
    pub text_margin: f32,
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
    pub outer_margin: u16,
    pub inner_margin: u16,
    pub text_margin: u16,
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
        let line_height = i32::conv_ceil(
            kas::text::fonts::fonts()
                .get_first_face(font_id)
                .height(dpem),
        );

        let outer_margin = (params.outer_margin * scale_factor).cast_nearest();
        let inner_margin = (params.inner_margin * scale_factor).cast_nearest();
        let text_margin = (params.text_margin * scale_factor).cast_nearest();
        let frame = (params.frame_size * scale_factor).cast_nearest();
        Dimensions {
            scale_factor,
            dpp,
            pt_size,
            font_marker_width: (1.6 * scale_factor).round().max(1.0),
            line_height,
            min_line_length: (8.0 * dpem).cast_nearest(),
            outer_margin,
            inner_margin,
            text_margin,
            frame,
            button_frame: (params.button_frame * scale_factor).cast_nearest(),
            checkbox: i32::conv_nearest(9.0 * dpp) + 2 * (i32::from(inner_margin) + frame),
            scrollbar: Size::from(params.scrollbar_size * scale_factor),
            slider: Size::from(params.slider_size * scale_factor),
            progress_bar: Size::from(params.progress_bar * scale_factor),
        }
    }
}

/// A convenient implementation of [`crate::Window`]
pub struct DimensionsWindow {
    pub dims: Dimensions,
    pub fonts: Rc<LinearMap<TextClass, FontId>>,
}

impl DimensionsWindow {
    pub fn new(
        dims: DimensionsParams,
        pt_size: f32,
        scale_factor: f32,
        fonts: Rc<LinearMap<TextClass, FontId>>,
    ) -> Self {
        DimensionsWindow {
            dims: Dimensions::new(dims, pt_size, scale_factor),
            fonts,
        }
    }

    pub fn update(&mut self, dims: DimensionsParams, pt_size: f32, scale_factor: f32) {
        self.dims = Dimensions::new(dims, pt_size, scale_factor);
    }
}

impl<DS: DrawableShared> crate::Window<DS> for DimensionsWindow {
    #[cfg(not(feature = "gat"))]
    type SizeHandle = SizeHandle<'static, DS>;
    #[cfg(feature = "gat")]
    type SizeHandle<'a> = SizeHandle<'a, DS>;

    #[cfg(not(feature = "gat"))]
    unsafe fn size_handle<'a>(&'a mut self, shared: &'a mut DrawShared<DS>) -> Self::SizeHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        let h: SizeHandle<'a, DS> = SizeHandle::new(self, shared);
        std::mem::transmute(h)
    }
    #[cfg(feature = "gat")]
    fn size_handle<'a>(&'a mut self, shared: &'a mut DrawShared<DS>) -> Self::SizeHandle<'a> {
        SizeHandle::new(self, shared)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct SizeHandle<'a, DS: DrawableShared> {
    w: &'a DimensionsWindow,
    shared: &'a mut DrawShared<DS>,
}

impl<'a, DS: DrawableShared> SizeHandle<'a, DS> {
    pub fn new(w: &'a DimensionsWindow, shared: &'a mut DrawShared<DS>) -> Self {
        SizeHandle { w, shared }
    }
}

impl<'a, DS: DrawableShared> draw::SizeHandle for SizeHandle<'a, DS> {
    fn draw_shared(&mut self) -> &mut dyn DrawSharedT {
        self.shared
    }

    fn scale_factor(&self) -> f32 {
        self.w.dims.scale_factor
    }

    fn pixels_from_points(&self, pt: f32) -> f32 {
        self.w.dims.dpp * pt
    }

    fn pixels_from_em(&self, em: f32) -> f32 {
        self.w.dims.dpp * self.w.dims.pt_size * em
    }

    fn frame(&self, _vert: bool) -> FrameRules {
        FrameRules::new_sym(self.w.dims.frame, 0, 0)
    }
    fn menu_frame(&self, vert: bool) -> FrameRules {
        let mut size = self.w.dims.frame;
        if vert {
            size /= 2;
        }
        FrameRules::new_sym(size, 0, 0)
    }
    fn separator(&self) -> Size {
        Size::splat(self.w.dims.frame)
    }

    fn nav_frame(&self, _vert: bool) -> FrameRules {
        FrameRules::new_sym(self.w.dims.inner_margin.into(), 0, 0)
    }

    fn inner_margin(&self) -> Size {
        Size::splat(self.w.dims.inner_margin.into())
    }

    fn outer_margins(&self) -> Margins {
        Margins::splat(self.w.dims.outer_margin)
    }

    fn line_height(&self, _: TextClass) -> i32 {
        self.w.dims.line_height
    }

    fn text_bound(
        &mut self,
        text: &mut dyn TextApi,
        class: TextClass,
        axis: AxisInfo,
    ) -> SizeRules {
        let required = text.update_env(|env| {
            if let Some(font_id) = self.w.fonts.get(&class).cloned() {
                env.set_font_id(font_id);
            }
            env.set_dpp(self.w.dims.dpp);
            env.set_pt_size(self.w.dims.pt_size);

            let mut bounds = kas::text::Vec2::INFINITY;
            if let Some(size) = axis.size_other_if_fixed(false) {
                bounds.1 = size.cast();
            } else if let Some(size) = axis.size_other_if_fixed(true) {
                bounds.0 = size.cast();
            }
            env.set_bounds(bounds);

            env.set_wrap(match class {
                TextClass::Label | TextClass::EditMulti | TextClass::LabelScroll => true,
                _ => false,
            });
        });

        let margin = self.w.dims.text_margin;
        let margins = (margin, margin);
        if axis.is_horizontal() {
            let bound = i32::conv_ceil(required.0);
            let min = self.w.dims.min_line_length;
            let (min, ideal) = match class {
                TextClass::Edit => (min, 2 * min),
                TextClass::EditMulti => (min, 3 * min),
                _ => (bound.min(min), bound.min(3 * min)),
            };
            // NOTE: using different variable-width stretch policies here can
            // cause problems (e.g. edit boxes greedily consuming too much
            // space). This is a hard layout problem; for now don't do this.
            let stretch = match class {
                TextClass::MenuLabel => Stretch::None,
                TextClass::Button => Stretch::Filler,
                _ => Stretch::Low,
            };
            SizeRules::new(min, ideal, margins, stretch)
        } else {
            let min = match class {
                TextClass::Label => i32::conv_ceil(required.1),
                TextClass::MenuLabel | TextClass::Button | TextClass::Edit => {
                    self.w.dims.line_height
                }
                TextClass::EditMulti | TextClass::LabelScroll => self.w.dims.line_height * 3,
            };
            let ideal = i32::conv_ceil(required.1).max(min);
            let stretch = match class {
                TextClass::EditMulti | TextClass::LabelScroll => Stretch::Low,
                _ => Stretch::None,
            };
            SizeRules::new(min, ideal, margins, stretch)
        }
    }

    fn edit_marker_width(&self) -> f32 {
        self.w.dims.font_marker_width
    }

    fn button_surround(&self, _vert: bool) -> FrameRules {
        let inner = self.w.dims.inner_margin.into();
        let outer = self.w.dims.outer_margin;
        FrameRules::new_sym(self.w.dims.frame, inner, outer)
    }

    fn edit_surround(&self, _vert: bool) -> FrameRules {
        let inner = self.w.dims.inner_margin.into();
        let outer = 0;
        FrameRules::new_sym(self.w.dims.frame, inner, outer)
    }

    fn checkbox(&self) -> Size {
        Size::splat(self.w.dims.checkbox)
    }

    #[inline]
    fn radiobox(&self) -> Size {
        self.checkbox()
    }

    fn scrollbar(&self) -> (Size, i32) {
        let size = self.w.dims.scrollbar;
        (size, 3 * size.0)
    }

    fn slider(&self) -> (Size, i32) {
        let size = self.w.dims.slider;
        (size, 5 * size.0)
    }

    fn progress_bar(&self) -> Size {
        self.w.dims.progress_bar
    }
}
