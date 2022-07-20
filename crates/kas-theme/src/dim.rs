// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Common implementation of [`kas::theme::ThemeSize`]

use linear_map::LinearMap;
use std::any::Any;
use std::f32;
use std::rc::Rc;

use crate::anim::AnimState;
use kas::cast::traits::*;
use kas::dir::Directional;
use kas::geom::{Rect, Size, Vec2};
use kas::layout::{AlignHints, AxisInfo, FrameRules, Margins, SizeRules, Stretch};
use kas::text::{fonts::FontId, Align, TextApi, TextApiExt};
use kas::theme::{Feature, FrameStyle, MarkStyle, TextClass, ThemeSize};

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
    /// Margin between text elements (horiz, vert)
    pub text_margin: (f32, f32),
    /// Frame size
    pub frame_size: f32,
    /// Popup frame size
    pub popup_frame_size: f32,
    /// MenuEntry frame size (horiz, vert)
    pub menu_frame: f32,
    /// Button frame size (non-flat outer region)
    pub button_frame: f32,
    /// CheckBox inner size in Points
    pub check_box_inner: f32,
    /// Larger size of a mark in Points
    pub mark: f32,
    /// Size of a slider handle; minimum size of a scroll bar handle
    pub handle_len: f32,
    /// Scroll bar minimum handle size
    pub scroll_bar_size: Vec2,
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
    pub dpem: f32,
    pub mark_line: f32,
    pub min_line_length: i32,
    pub outer_margin: u16,
    pub inner_margin: u16,
    pub text_margin: (u16, u16),
    pub frame: i32,
    pub popup_frame: i32,
    pub menu_frame: i32,
    pub button_frame: i32,
    pub check_box: i32,
    pub mark: i32,
    pub handle_len: i32,
    pub scroll_bar: Size,
    pub slider: Size,
    pub progress_bar: Size,
    pub shadow_a: Vec2,
    pub shadow_b: Vec2,
}

impl Dimensions {
    pub fn new(params: &Parameters, pt_size: f32, scale_factor: f32) -> Self {
        let dpp = scale_factor * (96.0 / 72.0);
        let dpem = dpp * pt_size;

        let outer_margin = (params.outer_margin * scale_factor).cast_nearest();
        let inner_margin = (params.inner_margin * scale_factor).cast_nearest();
        let text_m0 = (params.text_margin.0 * scale_factor).cast_nearest();
        let text_m1 = (params.text_margin.1 * scale_factor).cast_nearest();
        let frame = (params.frame_size * scale_factor).cast_nearest();
        let popup_frame = (params.popup_frame_size * scale_factor).cast_nearest();
        let menu_frame = (params.menu_frame * scale_factor).cast_nearest();

        let shadow_size = params.shadow_size * scale_factor;
        let shadow_offset = shadow_size * params.shadow_rel_offset;

        Dimensions {
            scale_factor,
            dpp,
            dpem,
            mark_line: (1.2 * dpp).round().max(1.0),
            min_line_length: (8.0 * dpem).cast_nearest(),
            outer_margin,
            inner_margin,
            text_margin: (text_m0, text_m1),
            frame,
            popup_frame,
            menu_frame,
            button_frame: (params.button_frame * scale_factor).cast_nearest(),
            check_box: i32::conv_nearest(params.check_box_inner * dpp)
                + 2 * (i32::from(inner_margin) + frame),
            mark: i32::conv_nearest(params.mark * dpp),
            handle_len: i32::conv_nearest(params.handle_len * dpp),
            scroll_bar: Size::conv_nearest(params.scroll_bar_size * scale_factor),
            slider: Size::conv_nearest(params.slider_size * scale_factor),
            progress_bar: Size::conv_nearest(params.progress_bar * scale_factor),
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
    fn size(&self) -> &dyn ThemeSize {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<D: 'static> ThemeSize for Window<D> {
    fn scale_factor(&self) -> f32 {
        self.dims.scale_factor
    }

    fn dpem(&self) -> f32 {
        self.dims.dpem
    }

    fn min_scroll_size(&self, axis_is_vertical: bool) -> i32 {
        if axis_is_vertical {
            (self.dims.dpem * 3.0).cast_ceil()
        } else {
            self.dims.min_line_length
        }
    }

    fn handle_len(&self) -> i32 {
        self.dims.handle_len
    }

    fn inner_margin(&self) -> Size {
        Size::splat(self.dims.inner_margin.into())
    }

    fn outer_margins(&self) -> Margins {
        Margins::splat(self.dims.outer_margin)
    }

    fn text_margins(&self) -> Margins {
        Margins::hv_splat(self.dims.text_margin)
    }

    fn feature(&self, feature: Feature, axis_is_vertical: bool) -> SizeRules {
        let dir_is_vertical;
        let mut size;
        let mut ideal_mul = 3;

        match feature {
            Feature::Separator => {
                return SizeRules::fixed_splat(self.dims.frame, 0);
            }
            Feature::Mark(MarkStyle::Point(dir)) => {
                let w = match dir.is_vertical() == axis_is_vertical {
                    true => self.dims.mark / 2 + i32::conv_ceil(self.dims.mark_line),
                    false => self.dims.mark + i32::conv_ceil(self.dims.mark_line),
                };
                return SizeRules::fixed_splat(w, self.dims.outer_margin);
            }
            Feature::CheckBox | Feature::RadioBox => {
                return SizeRules::fixed_splat(self.dims.check_box, self.dims.outer_margin);
            }
            Feature::ScrollBar(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.scroll_bar;
            }
            Feature::Slider(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.slider;
                ideal_mul = 5;
            }
            Feature::ProgressBar(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.progress_bar;
            }
        }

        let mut stretch = Stretch::High;
        if dir_is_vertical != axis_is_vertical {
            size = size.transpose();
            ideal_mul = 1;
            stretch = Stretch::None;
        }
        let m = self.dims.outer_margin;
        SizeRules::new(size.0, ideal_mul * size.0, (m, m), stretch)
    }

    fn align_feature(&self, feature: Feature, rect: Rect, hints: AlignHints) -> Rect {
        let mut ideal_size = rect.size;
        match feature {
            Feature::Separator => (), // has no direction so we cannot align
            Feature::Mark(_) => (),   // aligned when drawn instead
            Feature::CheckBox | Feature::RadioBox => {
                ideal_size = Size::splat(self.dims.check_box);
            }
            Feature::ScrollBar(dir) => {
                ideal_size.set_component(dir.flipped(), self.dims.scroll_bar.1);
            }
            Feature::Slider(dir) => {
                ideal_size.set_component(dir.flipped(), self.dims.slider.1);
            }
            Feature::ProgressBar(dir) => {
                ideal_size.set_component(dir.flipped(), self.dims.progress_bar.1);
            }
        }
        hints
            .complete(Align::Center, Align::Center)
            .aligned_rect(ideal_size, rect)
    }

    fn frame(&self, style: FrameStyle, _is_vert: bool) -> FrameRules {
        let inner = self.dims.inner_margin.into();
        match style {
            FrameStyle::InnerMargin => FrameRules::new_sym(0, 0, 0),
            FrameStyle::Frame => FrameRules::new_sym(self.dims.frame, 0, 0),
            FrameStyle::Popup => FrameRules::new_sym(self.dims.popup_frame, 0, 0),
            FrameStyle::MenuEntry => FrameRules::new_sym(self.dims.menu_frame, inner, 0),
            FrameStyle::NavFocus => FrameRules::new_sym(self.dims.inner_margin.into(), 0, 0),
            FrameStyle::Button => {
                let outer = self.dims.outer_margin;
                FrameRules::new_sym(self.dims.frame, 0, outer)
            }
            FrameStyle::EditBox => FrameRules::new_sym(self.dims.frame, inner, 0),
        }
    }

    fn text_bound(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules {
        let margin = match axis.is_horizontal() {
            true => self.dims.text_margin.0,
            false => self.dims.text_margin.1,
        };
        let margins = (margin, margin);

        // Note: for horizontal axis of Edit* classes, input text does not affect size rules.
        if axis.is_horizontal() {
            let min = self.dims.min_line_length;
            if let TextClass::Edit(multi) = class {
                let (min, ideal) = match multi {
                    false => (min, 2 * min),
                    true => (min, 3 * min),
                };
                return SizeRules::new(min, ideal, margins, Stretch::Low);
            } else if let TextClass::EditShort(_) = class {
                return SizeRules::new(min, min, margins, Stretch::Low);
            }
        }

        let mut env = text.env();

        if let Some(font_id) = self.fonts.get(&class).cloned() {
            env.font_id = font_id;
        }
        env.dpem = self.dims.dpem;
        // TODO(opt): setting horizontal alignment now could avoid re-wrapping
        // text. Unfortunately we don't know the desired alignment here.
        env.wrap = class.multi_line();
        if let Some(size) = axis.size_other_if_fixed(true) {
            env.bounds.0 = size.cast();
        }

        text.set_env(env);

        if axis.is_horizontal() {
            let min = self.dims.min_line_length;
            let limit = 3 * min;
            let bound = i32::conv_ceil(text.measure_width(limit.cast()));
            let (min, ideal) = (bound.min(min), bound.min(3 * min));

            // NOTE: using different variable-width stretch policies here can
            // cause problems (e.g. edit boxes greedily consuming too much
            // space). This is a hard layout problem; for now don't do this.
            let stretch = match class {
                TextClass::MenuLabel => Stretch::None,
                _ => Stretch::Low,
            };
            SizeRules::new(min, ideal, margins, stretch)
        } else {
            let bound = i32::conv_ceil(text.measure_height());
            let min = if matches!(class, TextClass::Label(true) | TextClass::AccelLabel(true)) {
                bound
            } else {
                let line_height = self.dims.dpem.cast_ceil();
                match class {
                    _ if class.single_line() => line_height,
                    TextClass::LabelScroll => bound.min(line_height * 3),
                    TextClass::Edit(true) => line_height * 3,
                    _ => unreachable!(),
                }
            };
            let ideal = bound.max(min);
            let stretch = match class {
                TextClass::LabelScroll | TextClass::Edit(true) => Stretch::Low,
                _ => Stretch::None,
            };
            SizeRules::new(min, ideal, margins, stretch)
        }
    }

    fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: (Align, Align),
    ) {
        let mut env = text.env();
        if let Some(font_id) = self.fonts.get(&class).cloned() {
            env.font_id = font_id;
        }
        env.dpem = self.dims.dpem;
        env.wrap = class.multi_line();
        env.align = align;
        env.bounds = size.cast();
        text.update_env(env);
    }
}
