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
    // NOTE: visual thickness is (button_frame * scale_factor).round() * (1 - BG_SHRINK_FACTOR)
    button_frame: 2.4,
    checkbox_inner: 5.0,
    scrollbar_size: Vec2::splat(8.0),
    slider_size: Vec2(16.0, 16.0),
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
    unsafe fn draw_handle(
        &self,
        draw: DrawIface<DS>,
        window: &mut Self::Window,
    ) -> Self::DrawHandle {
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

impl<'a, DS: DrawSharedImpl> draw::DrawHandle for DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    fn size_handle(&mut self) -> &mut dyn SizeHandle {
        self.window
    }

    fn draw_device(&mut self) -> &mut dyn Draw {
        &mut self.draw
    }

    fn new_pass(
        &mut self,
        inner_rect: Rect,
        offset: Offset,
        class: PassType,
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        let mut outer_rect = inner_rect;
        if class == PassType::Overlay {
            outer_rect = inner_rect.expand(self.window.dims.frame);
        }
        let mut draw = self.draw.new_pass(outer_rect, offset, class);

        if class == PassType::Overlay {
            let outer = Quad::from(outer_rect + offset);
            let inner = Quad::from(inner_rect + offset);
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
        self.draw.get_clip_rect()
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

    fn text(&mut self, pos: Coord, text: &TextDisplay, _: TextClass) {
        let pos = pos;
        let col = self.cols.text;
        self.draw.text(pos.into(), text, col);
    }

    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, _: TextClass) {
        self.draw.text_col_effects(
            (pos).into(),
            text.display(),
            self.cols.text,
            text.effect_tokens(),
        );
    }

    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, _: TextClass) {
        let pos = Vec2::from(pos);
        let col = self.cols.text;
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
        _: TextClass,
    ) {
        let pos = Vec2::from(pos);
        let col = self.cols.text;
        let sel_col = self.cols.text_over(self.cols.text_sel_bg);

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
                aux: sel_col,
            },
            Effect {
                start: range.end.cast(),
                flags: Default::default(),
                aux: col,
            },
        ];
        self.draw.text_effects(pos, text, &effects);
    }

    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, _: TextClass, byte: usize) {
        let width = self.window.dims.font_marker_width;
        let pos = Vec2::from(pos);

        let mut col = self.cols.nav_focus;
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
            col = col.average();
        }
    }

    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        if let Some(col) = self.cols.menu_entry(state) {
            let quad = Quad::from(rect);
            self.draw.rect(quad, col);
        }
    }

    fn button(&mut self, rect: Rect, col: Option<color::Rgb>, state: InputState) {
        let outer = Quad::from(rect);

        let col = if state.nav_focus && !state.disabled {
            self.cols.accent_soft
        } else {
            col.map(|c| c.into()).unwrap_or(self.cols.background)
        };
        let col = ColorsLinear::adjust_for_state(col, state);
        if col != self.cols.background {
            let inner = outer.shrink(self.window.dims.button_frame as f32 * BG_SHRINK_FACTOR);
            self.draw.rect(inner, col);
        }

        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        let inner = outer.shrink(self.window.dims.button_frame as f32);
        self.draw.rounded_frame(outer, inner, BG_SHRINK_FACTOR, col);
    }

    fn edit_box(&mut self, rect: Rect, mut state: InputState) {
        let outer = Quad::from(rect);

        state.depress = false;
        let col = match state.error {
            true => self.cols.edit_bg_error,
            false => self.cols.edit_bg,
        };
        let col = ColorsLinear::adjust_for_state(col, state);
        if col != self.cols.background {
            let inner = outer.shrink(self.window.dims.button_frame as f32 * BG_SHRINK_FACTOR);
            self.draw.rect(inner, col);
        }

        let inner = outer.shrink(self.window.dims.button_frame as f32);
        self.draw
            .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);

        let col = if state.nav_focus {
            self.cols.nav_focus
        } else {
            // TODO: should this be a dedicated colour?
            ColorsLinear::adjust_for_state(self.cols.accent.average(), state)
        };
        let r = 0.5 * self.window.dims.button_frame as f32;
        let y = outer.b.1 - r;
        let a = Vec2(outer.a.0 + r, y);
        let b = Vec2(outer.b.0 - r, y);
        self.draw.rounded_line(a, b, r, col);
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let outer = Quad::from(rect);

        let col = ColorsLinear::adjust_for_state(self.cols.background, state);
        if col != self.cols.background {
            let inner = outer.shrink(self.window.dims.button_frame as f32 * BG_SHRINK_FACTOR);
            self.draw.rect(inner, col);
        }

        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        let inner = outer.shrink(self.window.dims.button_frame as f32);
        self.draw.rounded_frame(outer, inner, BG_SHRINK_FACTOR, col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let inner = inner.shrink((2 * self.window.dims.inner_margin) as f32);
            self.draw.rect(inner, col);
        }
    }

    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let outer = Quad::from(rect);

        let col = ColorsLinear::adjust_for_state(self.cols.background, state);
        if col != self.cols.background {
            self.draw.circle(outer, 0.0, col);
        }

        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        const F: f32 = 2.0 * (1.0 - BG_SHRINK_FACTOR); // match checkbox frame
        let r = 1.0 - F * self.window.dims.button_frame as f32 / rect.size.0 as f32;
        self.draw.circle(outer, r, col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let r = self.window.dims.button_frame + 2 * self.window.dims.inner_margin as i32;
            let inner = outer.shrink(r as f32);
            self.draw.circle(inner, 0.0, col);
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, _dir: Direction, state: InputState) {
        // track
        let outer = Quad::from(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let mut col = self.cols.frame;
        col.a = 0.5; // HACK
        self.draw.rounded_frame(outer, inner, 0.0, col);

        // handle
        let outer = Quad::from(h_rect);
        let r = outer.size().min_comp() * 0.125;
        let outer = outer.shrink(r);
        let inner = outer.shrink(3.0 * r);
        let col = ColorsLinear::adjust_for_state(self.cols.frame, state);
        self.draw.rounded_frame(outer, inner, 0.0, col);
    }

    fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
        // track
        let mut outer = Quad::from(rect);
        let mid = Vec2::from(h_rect.pos + h_rect.size / 2);
        let (mut first, mut second);
        if dir.is_horizontal() {
            outer = outer.shrink_vec(Vec2(0.0, outer.size().1 * (1.0 / 3.0)));
            first = outer;
            second = outer;
            first.b.0 = mid.0;
            second.a.0 = mid.0;
        } else {
            outer = outer.shrink_vec(Vec2(outer.size().0 * (1.0 / 3.0), 0.0));
            first = outer;
            second = outer;
            first.b.1 = mid.1;
            second.a.1 = mid.1;
        };

        let dist = outer.size().min_comp() / 2.0;
        let inner = first.shrink(dist);
        self.draw.rounded_frame(first, inner, 0.0, self.cols.accent);
        let inner = second.shrink(dist);
        self.draw
            .rounded_frame(second, inner, 1.0 / 3.0, self.cols.frame);

        // handle; force it to be square
        let size = Size::splat(h_rect.size.0.min(h_rect.size.1));
        let offset = Offset::from((h_rect.size - size) / 2);
        let outer = Quad::from(Rect::new(h_rect.pos + offset, size));

        let col = if state.nav_focus && !state.disabled {
            self.cols.accent_soft
        } else {
            self.cols.background
        };
        let col = ColorsLinear::adjust_for_state(col, state);
        self.draw.circle(outer, 0.0, col);
        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        self.draw.circle(outer, 14.0 / 16.0, col);
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
        let col = self.cols.accent_soft;
        self.draw.rect(inner, col);
    }

    fn image(&mut self, id: ImageId, rect: Rect) {
        let rect = Quad::from(rect);
        self.draw.image(id, rect);
    }
}
