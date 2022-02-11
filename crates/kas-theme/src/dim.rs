// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Common implementation of [`kas::theme::SizeHandle`]

use linear_map::LinearMap;
use std::any::Any;
use std::f32;
use std::rc::Rc;

use crate::anim::AnimState;
use kas::cast::{Cast, CastFloat, ConvFloat};
use kas::geom::{Size, Vec2};
use kas::layout::{AxisInfo, FrameRules, Margins, SizeRules, Stretch};
use kas::text::{fonts::FontId, Align, TextApi, TextApiExt};
use kas::theme::{SizeHandle, TextClass};

/// Parameterisation of [`Dimensions`]
///
/// All dimensions are multiplied by the DPI factor, then rounded to the
/// nearest integer. Example: `(2.0 * 1.25).round() = 3.0`.
#[derive(Clone, Debug)]
pub struct Parameters {
    /// Space between elements
    pub outer_margin: f32,
    /// Margin inside a frame before contents
    pub inner_margin: f32,
    /// Margin around frames and seperators
    pub frame_margin: f32,
    /// Margin between text elements (horiz, vert)
    pub text_margin: (f32, f32),
    /// Frame size
    pub frame_size: f32,
    /// Button frame size (non-flat outer region)
    pub button_frame: f32,
    /// Checkbox inner size in Points
    pub checkbox_inner: f32,
    /// Scrollbar minimum handle size
    pub scrollbar_size: Vec2,
    /// Slider minimum handle size
    pub slider_size: Vec2,
    /// Progress bar size (horizontal)
    pub progress_bar: Vec2,
    /// Shadow size (average)
    pub shadow_size: Vec2,
    /// Proportional offset of shadow (range: -1..=1)
    pub shadow_rel_offset: Vec2,
}

/// Dimensions available within [`Window`]
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
    pub frame_margin: u16,
    pub text_margin: (u16, u16),
    pub frame: i32,
    pub button_frame: i32,
    pub checkbox: i32,
    pub scrollbar: Size,
    pub slider: Size,
    pub progress_bar: Size,
    pub shadow_a: Vec2,
    pub shadow_b: Vec2,
}

impl Dimensions {
    pub fn new(params: &Parameters, pt_size: f32, scale_factor: f32) -> Self {
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
        let frame_margin = (params.frame_margin * scale_factor).cast_nearest();
        let text_m0 = (params.text_margin.0 * scale_factor).cast_nearest();
        let text_m1 = (params.text_margin.1 * scale_factor).cast_nearest();
        let frame = (params.frame_size * scale_factor).cast_nearest();

        let shadow_size = params.shadow_size * scale_factor;
        let shadow_offset = shadow_size * params.shadow_rel_offset;

        Dimensions {
            scale_factor,
            dpp,
            pt_size,
            font_marker_width: (1.6 * scale_factor).round().max(1.0),
            line_height,
            min_line_length: (8.0 * dpem).cast_nearest(),
            outer_margin,
            inner_margin,
            frame_margin,
            text_margin: (text_m0, text_m1),
            frame,
            button_frame: (params.button_frame * scale_factor).cast_nearest(),
            checkbox: i32::conv_nearest(params.checkbox_inner * dpp)
                + 2 * (i32::from(inner_margin) + frame),
            scrollbar: Size::from(params.scrollbar_size * scale_factor),
            slider: Size::from(params.slider_size * scale_factor),
            progress_bar: Size::from(params.progress_bar * scale_factor),
            shadow_a: shadow_offset - shadow_size,
            shadow_b: shadow_offset + shadow_size,
        }
    }
}

/// A convenient implementation of [`crate::Window`]
pub struct Window<D> {
    pub dims: Dimensions,
    pub fonts: Rc<LinearMap<TextClass, FontId>>,
    pub anim: AnimState<D>,
}

impl<D> Window<D> {
    pub fn new(
        dims: &Parameters,
        config: &crate::Config,
        scale_factor: f32,
        fonts: Rc<LinearMap<TextClass, FontId>>,
    ) -> Self {
        Window {
            dims: Dimensions::new(dims, config.font_size(), scale_factor),
            fonts,
            anim: AnimState::new(config),
        }
    }

    pub fn update(&mut self, dims: &Parameters, config: &crate::Config, scale_factor: f32) {
        self.dims = Dimensions::new(dims, config.font_size(), scale_factor);
    }
}

impl<D: 'static> crate::Window for Window<D> {
    fn size_handle(&self) -> &dyn SizeHandle {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<D: 'static> SizeHandle for Window<D> {
    fn scale_factor(&self) -> f32 {
        self.dims.scale_factor
    }

    fn pixels_from_points(&self, pt: f32) -> f32 {
        self.dims.dpp * pt
    }

    fn pixels_from_em(&self, em: f32) -> f32 {
        self.dims.dpp * self.dims.pt_size * em
    }

    fn frame(&self, _vert: bool) -> FrameRules {
        FrameRules::new_sym(self.dims.frame, 0, self.dims.frame_margin)
    }
    fn menu_frame(&self, vert: bool) -> FrameRules {
        let size = match vert {
            false => self.dims.text_margin.0,
            true => self.dims.text_margin.1,
        };
        FrameRules::new_sym(size.cast(), 0, 0)
    }
    fn separator(&self) -> Size {
        Size::splat(self.dims.frame)
    }

    fn nav_frame(&self, _vert: bool) -> FrameRules {
        FrameRules::new_sym(self.dims.inner_margin.into(), 0, 0)
    }

    fn inner_margin(&self) -> Size {
        Size::splat(self.dims.inner_margin.into())
    }

    fn outer_margins(&self) -> Margins {
        Margins::splat(self.dims.outer_margin)
    }

    fn frame_margins(&self) -> Margins {
        Margins::splat(self.dims.frame_margin)
    }

    fn text_margins(&self) -> Margins {
        Margins::hv_splat(self.dims.text_margin)
    }

    fn line_height(&self, _: TextClass) -> i32 {
        self.dims.line_height
    }

    fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules {
        // Note: for horizontal axis of Edit* classes, input text does not affect size rules.
        // We must set env at least once, but do for vertical axis anyway.
        let mut required = None;
        if !axis.is_horizontal() || !matches!(class, TextClass::Edit | TextClass::EditMulti) {
            required = Some(text.update_env(|env| {
                if let Some(font_id) = self.fonts.get(&class).cloned() {
                    env.set_font_id(font_id);
                }
                env.set_dpp(self.dims.dpp);
                env.set_pt_size(self.dims.pt_size);

                let mut bounds = kas::text::Vec2::INFINITY;
                if let Some(size) = axis.size_other_if_fixed(false) {
                    bounds.1 = size.cast();
                } else if let Some(size) = axis.size_other_if_fixed(true) {
                    bounds.0 = size.cast();
                }
                env.set_bounds(bounds);
                env.set_align((Align::TL, Align::TL)); // force top-left alignment for sizing

                env.set_wrap(matches!(
                    class,
                    TextClass::Label | TextClass::EditMulti | TextClass::LabelScroll
                ));
            }));
        }

        let margin = match axis.is_horizontal() {
            true => self.dims.text_margin.0,
            false => self.dims.text_margin.1,
        };
        let margins = (margin, margin);
        if axis.is_horizontal() {
            let min = self.dims.min_line_length;
            let (min, ideal) = match class {
                TextClass::Edit => (min, 2 * min),
                TextClass::EditMulti => (min, 3 * min),
                _ => {
                    let bound = i32::conv_ceil(required.unwrap().0);
                    (bound.min(min), bound.min(3 * min))
                }
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
            let bound = i32::conv_ceil(required.unwrap().1);
            let min = match class {
                TextClass::Label => bound,
                TextClass::MenuLabel | TextClass::Button | TextClass::Edit => self.dims.line_height,
                TextClass::EditMulti | TextClass::LabelScroll => self.dims.line_height * 3,
            };
            let ideal = bound.max(min);
            let stretch = match class {
                TextClass::EditMulti | TextClass::LabelScroll => Stretch::Low,
                _ => Stretch::None,
            };
            SizeRules::new(min, ideal, margins, stretch)
        }
    }

    fn text_cursor_width(&self) -> f32 {
        self.dims.font_marker_width
    }

    fn button_surround(&self, _vert: bool) -> FrameRules {
        let inner = self.dims.inner_margin.into();
        let outer = self.dims.outer_margin;
        FrameRules::new_sym(self.dims.frame, inner, outer)
    }

    fn edit_surround(&self, _vert: bool) -> FrameRules {
        let inner = self.dims.inner_margin.into();
        let outer = 0;
        FrameRules::new_sym(self.dims.frame, inner, outer)
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
        (size, 3 * size.0)
    }

    fn slider(&self) -> (Size, i32) {
        let size = self.dims.slider;
        (size, 5 * size.0)
    }

    fn progress_bar(&self) -> Size {
        self.dims.progress_bar
    }
}
