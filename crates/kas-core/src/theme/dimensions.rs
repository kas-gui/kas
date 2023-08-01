// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! A shared implementation of [`ThemeSize`]

use linear_map::LinearMap;
use std::any::Any;
use std::f32;
use std::rc::Rc;

use super::anim::AnimState;
use super::{Config, Feature, FrameStyle, MarginStyle, MarkStyle, TextClass, ThemeSize};
use crate::cast::traits::*;
use crate::dir::Directional;
use crate::geom::{Rect, Size, Vec2};
use crate::layout::{AlignPair, AxisInfo, FrameRules, Margins, SizeRules, Stretch};
use crate::text::{fonts::FontId, TextApi, TextApiExt};

crate::impl_scope! {
    /// Parameterisation of [`Dimensions`]
    ///
    /// All dimensions are multiplied by the DPI factor, then rounded to the
    /// nearest integer. Example: `(2.0 * 1.25).round() = 3.0`.
    ///
    /// "Reasonable defaults" are provided for all values.
    #[derive(Clone, Debug)]
    #[impl_default]
    pub struct Parameters {
        /// Inner margin, used to draw highlight/selection boxes
        ///
        /// Guide size: 1px at 100%, 2px at 125%, 2px at 150%, 3px at 200%.
        ///
        /// This is the smallest of the fixed margin sizes, and only really
        /// useful to reserve space for drawing selection boxes.
        pub m_inner: f32 = 1.25,
        /// A small margin for inner layout
        ///
        /// Guide size: 2px at 100%, 3px at 125%, 4px at 150%, 5px at 200%.
        pub m_tiny: f32 = 2.4,
        /// Small external margin size
        ///
        /// Guide size: 4px at 100%, 5px at 125%, 7px at 150%, 9px at 200%.
        pub m_small: f32 = 4.35,
        /// Large margin, used between elements such as buttons
        ///
        /// Guide size: 7px at 100%, 9px at 125%, 11px at 150%, 15px at 200%.
        pub m_large: f32 = 7.4,
        /// Margin around text elements (horiz, vert)
        pub m_text: (f32, f32) = (3.3, 0.8),
        /// Frame size
        pub frame: f32 = 2.4,
        /// Popup frame size
        pub popup_frame: f32 = 0.0,
        /// MenuEntry frame size (horiz, vert)
        pub menu_frame: f32 = 2.4,
        /// Button frame size (non-flat outer region)
        pub button_frame: f32 = 2.4,
        /// Button inner margin (also drawable)
        pub button_inner: f32 = 0.0,
        /// CheckBox size
        pub check_box: f32 = 18.0,
        /// Larger size of a mark
        pub mark: f32 = 10.0,
        /// Length of a slider handle; minimum length of a scroll bar handle
        pub handle_len: f32 = 16.0,
        /// Minimum size for a horizontal scroll bar
        pub scroll_bar_size: Vec2 = Vec2(24.0, 8.0),
        /// Minimum size for a horizontal slider
        pub slider_size: Vec2 = Vec2(24.0, 12.0),
        /// Minimum size for a horizontal progress bar
        pub progress_bar: Vec2 = Vec2(24.0, 8.0),
        /// Shadow size (average)
        pub shadow_size: Vec2 = Vec2::ZERO,
        /// Proportional offset of shadow (range: -1..=1)
        pub shadow_rel_offset: Vec2 = Vec2::ZERO,
    }
}

/// Dimensions available within [`Window`]
#[derive(Clone, Debug)]
pub struct Dimensions {
    /// Scale factor
    pub scale: f32,
    pub dpem: f32,
    pub mark_line: f32,
    pub min_line_length: i32,
    pub m_inner: u16,
    pub m_tiny: u16,
    pub m_small: u16,
    pub m_large: u16,
    pub m_text: (u16, u16),
    pub frame: i32,
    pub popup_frame: i32,
    pub menu_frame: i32,
    pub button_frame: i32,
    pub button_inner: u16,
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
    pub fn new(params: &Parameters, pt_size: f32, scale: f32) -> Self {
        let dpp = scale * (96.0 / 72.0);
        let dpem = dpp * pt_size;

        let text_m0 = (params.m_text.0 * scale).cast_nearest();
        let text_m1 = (params.m_text.1 * scale).cast_nearest();

        let shadow_size = params.shadow_size * scale;
        let shadow_offset = shadow_size * params.shadow_rel_offset;

        Dimensions {
            scale,
            dpem,
            mark_line: (1.6 * scale).round().max(1.0),
            min_line_length: (8.0 * dpem).cast_nearest(),
            m_inner: (params.m_inner * scale).cast_nearest(),
            m_tiny: (params.m_tiny * scale).cast_nearest(),
            m_small: (params.m_small * scale).cast_nearest(),
            m_large: (params.m_large * scale).cast_nearest(),
            m_text: (text_m0, text_m1),
            frame: (params.frame * scale).cast_nearest(),
            popup_frame: (params.popup_frame * scale).cast_nearest(),
            menu_frame: (params.menu_frame * scale).cast_nearest(),
            button_frame: (params.button_frame * scale).cast_nearest(),
            button_inner: (params.button_inner * scale).cast_nearest(),
            check_box: i32::conv_nearest(params.check_box * scale),
            mark: i32::conv_nearest(params.mark * scale),
            handle_len: i32::conv_nearest(params.handle_len * scale),
            scroll_bar: Size::conv_nearest(params.scroll_bar_size * scale),
            slider: Size::conv_nearest(params.slider_size * scale),
            progress_bar: Size::conv_nearest(params.progress_bar * scale),
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
        config: &Config,
        scale: f32,
        fonts: Rc<LinearMap<TextClass, FontId>>,
    ) -> Self {
        Window {
            dims: Dimensions::new(dims, config.font_size(), scale),
            fonts,
            anim: AnimState::new(config),
        }
    }

    pub fn update(&mut self, dims: &Parameters, config: &Config, scale: f32) {
        self.dims = Dimensions::new(dims, config.font_size(), scale);
    }
}

impl<D: 'static> super::Window for Window<D> {
    fn size(&self) -> &dyn ThemeSize {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<D: 'static> ThemeSize for Window<D> {
    fn scale_factor(&self) -> f32 {
        self.dims.scale
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

    fn scroll_bar_width(&self) -> i32 {
        self.dims.scroll_bar.1
    }

    fn margins(&self, style: MarginStyle) -> Margins {
        match style {
            MarginStyle::None => Margins::ZERO,
            MarginStyle::Inner => Margins::splat(self.dims.m_inner),
            MarginStyle::Tiny => Margins::splat(self.dims.m_tiny),
            MarginStyle::Small => Margins::splat(self.dims.m_small),
            MarginStyle::Large => Margins::splat(self.dims.m_large),
            MarginStyle::Text => Margins::hv_splat(self.dims.m_text),
            MarginStyle::Px(px) => Margins::splat(u16::conv_nearest(px * self.dims.scale)),
            MarginStyle::Em(em) => Margins::splat(u16::conv_nearest(em * self.dims.dpem)),
        }
    }

    fn feature(&self, feature: Feature, axis_is_vertical: bool) -> SizeRules {
        let dir_is_vertical;
        let mut size;
        let mut ideal_mul = 3;
        let m;

        match feature {
            Feature::Separator => {
                return SizeRules::fixed_splat(self.dims.frame, 0);
            }
            Feature::Mark(MarkStyle::Point(dir)) => {
                let w = match dir.is_vertical() == axis_is_vertical {
                    true => self.dims.mark / 2 + i32::conv_ceil(self.dims.mark_line),
                    false => self.dims.mark + i32::conv_ceil(self.dims.mark_line),
                };
                return SizeRules::fixed_splat(w, self.dims.m_tiny);
            }
            Feature::Mark(MarkStyle::X) => {
                let w = self.dims.mark + i32::conv_ceil(self.dims.mark_line);
                return SizeRules::fixed_splat(w, self.dims.m_tiny);
            }
            Feature::CheckBox | Feature::RadioBox => {
                return SizeRules::fixed_splat(self.dims.check_box, self.dims.m_small);
            }
            Feature::ScrollBar(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.scroll_bar;
                m = 0;
            }
            Feature::Slider(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.slider;
                ideal_mul = 5;
                m = self.dims.m_large;
            }
            Feature::ProgressBar(dir) => {
                dir_is_vertical = dir.is_vertical();
                size = self.dims.progress_bar;
                m = self.dims.m_large;
            }
        }

        let mut stretch = Stretch::High;
        if dir_is_vertical != axis_is_vertical {
            size = size.transpose();
            ideal_mul = 1;
            stretch = Stretch::None;
        }
        SizeRules::new(size.0, ideal_mul * size.0, (m, m), stretch)
    }

    fn align_feature(&self, feature: Feature, rect: Rect, align: AlignPair) -> Rect {
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
        align.aligned_rect(ideal_size, rect)
    }

    fn frame(&self, style: FrameStyle, is_vert: bool) -> FrameRules {
        let outer = self.dims.m_large;
        match style {
            FrameStyle::None => FrameRules::ZERO,
            FrameStyle::Frame => FrameRules::new_sym(self.dims.frame, 0, outer),
            FrameStyle::Window => FrameRules::new_sym(self.dims.frame, 0, 0),
            FrameStyle::Popup => FrameRules::new_sym(self.dims.popup_frame, 0, 0),
            FrameStyle::MenuEntry => FrameRules::new_sym(self.dims.menu_frame, 0, 0),
            FrameStyle::NavFocus => FrameRules::new_sym(0, self.dims.m_inner, 0),
            FrameStyle::Button | FrameStyle::Tab => {
                if is_vert && style == FrameStyle::Tab {
                    FrameRules::new(
                        self.dims.button_frame,
                        (self.dims.button_inner, 0),
                        (outer, 0),
                    )
                } else {
                    FrameRules::new_sym(self.dims.button_frame, self.dims.button_inner, outer)
                }
            }
            FrameStyle::EditBox => FrameRules::new_sym(self.dims.frame, 0, outer),
        }
    }

    fn line_height(&self, class: TextClass) -> i32 {
        let font_id = self.fonts.get(&class).cloned().unwrap_or_default();
        crate::text::fonts::fonts()
            .get_first_face(font_id)
            .expect("invalid font_id")
            .height(self.dims.dpem)
            .cast_ceil()
    }

    fn text_rules(&self, text: &mut dyn TextApi, class: TextClass, axis: AxisInfo) -> SizeRules {
        let margin = match axis.is_horizontal() {
            true => self.dims.m_text.0,
            false => self.dims.m_text.1,
        };
        let margins = (margin, margin);

        let mut env = text.env();

        // TODO(opt): maybe font look-up should only happen during configure?
        if let Some(font_id) = self.fonts.get(&class).cloned() {
            env.font_id = font_id;
        }
        env.dpem = self.dims.dpem;
        // TODO(opt): setting horizontal alignment now could avoid re-wrapping
        // text. Unfortunately we don't know the desired alignment here.
        let wrap = class.multi_line();
        env.wrap = wrap;
        let align = axis.align_or_default();
        if axis.is_horizontal() {
            env.align.0 = align;
        } else {
            env.align.1 = align;
        }
        if let Some(size) = axis.size_other_if_fixed(true) {
            env.bounds.0 = size.cast();
        }

        text.set_env(env);

        if axis.is_horizontal() {
            if wrap {
                let min = self.dims.min_line_length;
                let limit = 2 * min;
                let bound: i32 = text
                    .measure_width(limit.cast())
                    .expect("invalid font_id")
                    .cast_ceil();

                // NOTE: using different variable-width stretch policies here can
                // cause problems (e.g. edit boxes greedily consuming too much
                // space). This is a hard layout problem; for now don't do this.
                SizeRules::new(bound.min(min), bound.min(limit), margins, Stretch::Filler)
            } else {
                let bound: i32 = text
                    .measure_width(f32::INFINITY)
                    .expect("invalid font_id")
                    .cast_ceil();
                SizeRules::new(bound, bound, margins, Stretch::Filler)
            }
        } else {
            let bound: i32 = text.measure_height().expect("invalid font_id").cast_ceil();

            let line_height = self.dims.dpem.cast_ceil();
            let min = bound.max(line_height);
            SizeRules::new(min, min, margins, Stretch::Filler)
        }
    }

    fn text_set_size(
        &self,
        text: &mut dyn TextApi,
        class: TextClass,
        size: Size,
        align: Option<AlignPair>,
    ) {
        let mut env = text.env();
        if let Some(font_id) = self.fonts.get(&class).cloned() {
            env.font_id = font_id;
        }
        env.dpem = self.dims.dpem;
        env.wrap = class.multi_line();
        if let Some(align) = align {
            env.align = align.into();
        }
        env.bounds = size.cast();
        text.update_env(env).expect("invalid font_id");
    }
}
