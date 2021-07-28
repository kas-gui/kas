// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use linear_map::LinearMap;
use std::f32;
use std::ops::Range;
use std::rc::Rc;

use crate::{dim, ColorsLinear, Config, Theme};
use kas::cast::Cast;
use kas::dir::{Direction, Directional};
use kas::draw::{self, color::Rgba, *};
use kas::geom::*;
use kas::text::format::FormattableText;
use kas::text::{fonts, AccelString, Effect, Text, TextApi, TextDisplay};
use kas::TkAction;

// Used to ensure a rectangular background is inside a circular corner.
// Also the maximum inner radius of circular borders to overlap with this rect.
const BG_SHRINK_FACTOR: f32 = 1.0 - std::f32::consts::FRAC_1_SQRT_2;

/// A theme with flat (unshaded) rendering
#[derive(Clone, Debug)]
pub struct FlatTheme {
    pub(crate) config: Config,
    pub(crate) cols: ColorsLinear,
    pub(crate) fonts: Option<Rc<LinearMap<TextClass, fonts::FontId>>>,
}

impl Default for FlatTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatTheme {
    /// Construct
    #[inline]
    pub fn new() -> Self {
        FlatTheme {
            config: Default::default(),
            cols: ColorsLinear::default(),
            fonts: None,
        }
    }

    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    #[inline]
    pub fn with_font_size(mut self, pt_size: f32) -> Self {
        self.config.set_font_size(pt_size);
        self
    }

    /// Set the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    #[inline]
    pub fn with_colours(mut self, scheme: &str) -> Self {
        self.config.set_active_scheme(scheme);
        if let Some(scheme) = self.config.get_color_scheme(scheme) {
            self.cols = scheme.into();
        }
        self
    }
}

const DIMS: dim::Parameters = dim::Parameters {
    outer_margin: 8.0,
    inner_margin: 1.2,
    text_margin: 2.0,
    frame_size: 4.0,
    button_frame: 6.0,
    scrollbar_size: Vec2::splat(8.0),
    slider_size: Vec2(12.0, 25.0),
    progress_bar: Vec2::splat(12.0),
};

pub struct DrawHandle<'a, DS: DrawSharedImpl> {
    pub(crate) draw: DrawIface<'a, DS>,
    pub(crate) window: &'a mut dim::Window,
    pub(crate) cols: &'a ColorsLinear,
}

impl<DS: DrawSharedImpl> Theme<DS> for FlatTheme
where
    DS::Draw: DrawRoundedImpl,
{
    type Config = Config;
    type Window = dim::Window;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = DrawHandle<'static, DS>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = DrawHandle<'a, DS>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        std::borrow::Cow::Borrowed(&self.config)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        let action = self.config.apply_config(config);
        if let Some(scheme) = self.config.get_active_scheme() {
            self.cols = scheme.into();
        }
        action
    }

    fn init(&mut self, _shared: &mut SharedState<DS>) {
        let fonts = fonts::fonts();
        if let Err(e) = fonts.select_default() {
            panic!("Error loading font: {}", e);
        }
        self.fonts = Some(Rc::new(
            self.config
                .iter_fonts()
                .filter_map(|(c, s)| fonts.select_font(s).ok().map(|id| (*c, id)))
                .collect(),
        ));
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        let fonts = self.fonts.as_ref().unwrap().clone();
        dim::Window::new(DIMS, self.config.font_size(), dpi_factor, fonts)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        window.update(DIMS, self.config.font_size(), dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(&self, draw: DrawIface<DS>, window: &mut Self::Window) -> Self::DrawHandle {
        unsafe fn extend_lifetime<'b, T: ?Sized>(r: &'b T) -> &'static T {
            std::mem::transmute::<&'b T, &'static T>(r)
        }
        unsafe fn extend_lifetime_mut<'b, T: ?Sized>(r: &'b mut T) -> &'static mut T {
            std::mem::transmute::<&'b mut T, &'static mut T>(r)
        }
        DrawHandle {
            draw: DrawIface {
                draw: extend_lifetime_mut(draw.draw),
                shared: extend_lifetime_mut(draw.shared),
                pass: draw.pass,
            },
            window: extend_lifetime_mut(window),
            cols: extend_lifetime(&self.cols),
        }
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle<'a> {
        DrawHandle {
            draw,
            window,
            cols: &self.cols,
        }
    }

    fn clear_color(&self) -> Rgba {
        self.cols.background
    }
}

impl ThemeApi for FlatTheme {
    fn set_font_size(&mut self, pt_size: f32) -> TkAction {
        self.config.set_font_size(pt_size);
        TkAction::RESIZE | TkAction::THEME_UPDATE
    }

    fn list_schemes(&self) -> Vec<&str> {
        self.config
            .color_schemes_iter()
            .map(|(name, _)| name)
            .collect()
    }

    fn set_scheme(&mut self, name: &str) -> TkAction {
        if name != self.config.active_scheme() {
            if let Some(scheme) = self.config.get_color_scheme(name) {
                self.cols = scheme.into();
                self.config.set_active_scheme(name);
                return TkAction::REDRAW;
            }
        }
        TkAction::empty()
    }
}

impl<'a, DS: DrawSharedImpl> DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    /// Draw an edit box with optional navigation highlight.
    /// Return the inner rect.
    ///
    /// - `outer`: define position via outer rect
    /// - `bg_col`: colour of background
    /// - `nav_col`: colour of navigation highlight, if visible
    fn draw_edit_box(&mut self, outer: Rect, bg_col: Rgba, nav_col: Option<Rgba>) -> Quad {
        let outer = Quad::from(outer);
        let inner1 = outer.shrink(self.window.dims.frame as f32 * BG_SHRINK_FACTOR);
        let inner2 = outer.shrink(self.window.dims.frame as f32);

        self.draw.rect(inner1, bg_col);

        // We draw over the inner rect, taking advantage of the fact that
        // rounded frames get drawn after flat rects.
        self.draw
            .rounded_frame(outer, inner2, BG_SHRINK_FACTOR, self.cols.frame);

        if let Some(col) = nav_col {
            self.draw.rounded_frame(inner1, inner2, 0.6, col);
        }

        inner2
    }

    /// Draw a handle (for slider, scrollbar)
    fn draw_handle(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect);
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.scrollbar_state(state);
        self.draw.rounded_frame(outer, inner, 0.0, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(thickness / 4.0);
            self.draw.rounded_frame(outer, inner, 0.6, col);
        }
    }
}

impl<'a, DS: DrawSharedImpl> draw::DrawHandle for DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    fn size_handle(&mut self) -> &mut dyn SizeHandle {
        self.window
    }

    fn draw_device<'b>(&'b mut self) -> &mut dyn DrawT {
        &mut self.draw
    }

    fn new_draw_pass(
        &mut self,
        mut rect: Rect,
        offset: Offset,
        class: PassType,
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        if class == PassType::Overlay {
            rect = rect.expand(self.window.dims.frame);
        }
        let mut draw = self.draw.new_draw_pass(rect, offset, class);

        if class == PassType::Overlay {
            let outer = draw.clip_rect();
            let inner = Quad::from(outer.shrink(self.window.dims.frame));
            let outer = Quad::from(outer);
            draw.rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
            let inner = outer.shrink(self.window.dims.frame as f32 * BG_SHRINK_FACTOR);
            draw.rect(inner, self.cols.background);
        }

        let mut handle = DrawHandle {
            window: self.window,
            draw,
            cols: self.cols,
        };
        f(&mut handle);
    }

    fn get_clip_rect(&self) -> Rect {
        self.draw.clip_rect()
    }

    fn outer_frame(&mut self, rect: Rect) {
        let outer = Quad::from(rect);
        let inner = outer.shrink(self.window.dims.frame as f32);
        self.draw
            .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::from(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        self.draw
            .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
    }

    fn nav_frame(&mut self, rect: Rect, state: InputState) {
        if let Some(col) = self.cols.nav_region(state) {
            let outer = Quad::from(rect);
            let inner = outer.shrink(self.window.dims.inner_margin as f32);
            self.draw.rounded_frame(outer, inner, 0.0, col);
        }
    }

    fn selection_box(&mut self, rect: Rect) {
        let inner = Quad::from(rect);
        let outer = inner.grow(self.window.dims.inner_margin.into());
        // TODO: this should use its own colour and a stippled pattern
        let col = self.cols.text_sel_bg;
        self.draw.frame(outer, inner, col);
    }

    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass) {
        let pos = pos;
        let col = self.cols.text_class(class);
        self.draw.text(pos.into(), text, col);
    }

    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.draw.text_col_effects(
            (pos).into(),
            text.display(),
            self.cols.text_class(class),
            text.effect_tokens(),
        );
    }

    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass) {
        let pos = Vec2::from(pos);
        let col = self.cols.text_class(class);
        if state {
            let effects = text.text().effect_tokens();
            self.draw.text_col_effects(pos, text.as_ref(), col, effects);
        } else {
            self.draw.text(pos, text.as_ref(), col);
        }
    }

    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    ) {
        let pos = Vec2::from(pos);
        let col = self.cols.text_class(class);

        // Draw background:
        for (p1, p2) in &text.highlight_lines(range.clone()) {
            let p1 = Vec2::from(*p1);
            let p2 = Vec2::from(*p2);
            let quad = Quad::with_coords(pos + p1, pos + p2);
            self.draw.rect(quad, self.cols.text_sel_bg);
        }

        let effects = [
            Effect {
                start: 0,
                flags: Default::default(),
                aux: col,
            },
            Effect {
                start: range.start.cast(),
                flags: Default::default(),
                aux: self.cols.text_sel,
            },
            Effect {
                start: range.end.cast(),
                flags: Default::default(),
                aux: col,
            },
        ];
        self.draw.text_effects(pos, text, &effects);
    }

    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        let width = self.window.dims.font_marker_width;
        let pos = Vec2::from(pos);

        let mut col = self.cols.text_class(class);
        for cursor in text.text_glyph_pos(byte).rev() {
            let mut p1 = pos + Vec2::from(cursor.pos);
            let mut p2 = p1;
            p1.1 -= cursor.ascent;
            p2.1 -= cursor.descent;
            p2.0 += width;
            let quad = Quad::with_coords(p1, p2);
            self.draw.rect(quad, col);

            if cursor.embedding_level() > 0 {
                // Add a hat to indicate directionality.
                let height = width;
                let quad = if cursor.is_ltr() {
                    Quad::with_coords(Vec2(p2.0, p1.1), Vec2(p2.0 + width, p1.1 + height))
                } else {
                    Quad::with_coords(Vec2(p1.0 - width, p1.1), Vec2(p1.0, p1.1 + height))
                };
                self.draw.rect(quad, col);
            }
            // hack to make secondary marker grey:
            col = self.cols.button_disabled;
        }
    }

    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        if let Some(col) = self.cols.menu_entry(state) {
            let quad = Quad::from(rect);
            self.draw.rect(quad, col);
        }
    }

    fn button(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect);
        let col = self.cols.button_state(state);

        let inner = outer.shrink(self.window.dims.button_frame as f32);
        self.draw.rounded_frame(outer, inner, 0.0, col);
        self.draw.rect(inner, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(self.window.dims.inner_margin as f32);
            self.draw.rounded_frame(outer, inner, 0.6, col);
        }
    }

    fn edit_box(&mut self, rect: Rect, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        self.draw_edit_box(rect, bg_col, self.cols.nav_region(state));
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let radius = inner.size().sum() * (1.0 / 16.0);
            let inner = inner.shrink(self.window.dims.inner_margin as f32 + radius);
            self.draw.rounded_line(inner.a, inner.b, radius, col);
            self.draw.rounded_line(inner.ab(), inner.ba(), radius, col);
        }
    }

    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let inner = inner.shrink(self.window.dims.inner_margin as f32);
            self.draw.circle(inner, 0.5, col);
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, _dir: Direction, state: InputState) {
        // track
        let outer = Quad::from(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.frame;
        self.draw.rounded_frame(outer, inner, 0.0, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
        // track
        let mut outer = Quad::from(rect);
        outer = match dir.is_horizontal() {
            true => outer.shrink_vec(Vec2(0.0, outer.size().1 * (3.0 / 8.0))),
            false => outer.shrink_vec(Vec2(outer.size().0 * (3.0 / 8.0), 0.0)),
        };
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.frame;
        self.draw.rounded_frame(outer, inner, 0.0, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn progress_bar(&mut self, rect: Rect, dir: Direction, _: InputState, value: f32) {
        let outer = Quad::from(rect);
        let mut inner = outer.shrink(self.window.dims.frame as f32);
        let col = self.cols.frame;
        self.draw.rounded_frame(outer, inner, BG_SHRINK_FACTOR, col);

        if dir.is_horizontal() {
            inner.b.0 = inner.a.0 + value * (inner.b.0 - inner.a.0);
        } else {
            inner.b.1 = inner.a.1 + value * (inner.b.1 - inner.a.1);
        }
        let col = self.cols.button;
        self.draw.rect(inner, col);
    }

    fn image(&mut self, id: ImageId, rect: Rect) {
        let rect = Quad::from(rect);
        self.draw.image(id, rect);
    }
}
