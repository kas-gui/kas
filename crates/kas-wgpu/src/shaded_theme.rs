// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shaded theme

use std::f32;
use std::ops::Range;
use std::time::Instant;

use crate::{DrawShaded, DrawShadedImpl};
use kas::cast::traits::*;
use kas::dir::{Direction, Directional};
use kas::draw::{color::Rgba, *};
use kas::event::EventState;
use kas::geom::*;
use kas::text::TextDisplay;
use kas::theme::dimensions as dim;
use kas::theme::{Background, ThemeControl, ThemeDraw, ThemeSize};
use kas::theme::{ColorsLinear, Config, FlatTheme, InputState, SimpleTheme, Theme};
use kas::theme::{FrameStyle, MarkStyle, TextClass};
use kas::{Action, Id};

/// A theme using simple shading to give apparent depth to elements
#[derive(Clone, Debug)]
pub struct ShadedTheme {
    base: SimpleTheme,
}

impl Default for ShadedTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadedTheme {
    /// Construct
    pub fn new() -> Self {
        let base = SimpleTheme::new();
        ShadedTheme { base }
    }

    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    #[must_use]
    pub fn with_font_size(mut self, pt_size: f32) -> Self {
        self.base = self.base.with_font_size(pt_size);
        self
    }

    /// Set the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    #[must_use]
    pub fn with_colours(mut self, scheme: &str) -> Self {
        self.base = self.base.with_colours(scheme);
        self
    }
}

fn dimensions() -> dim::Parameters {
    dim::Parameters {
        m_large: 5.0,
        m_text: (3.4, 0.8),
        frame: 5.0,
        button_frame: 2.0,
        button_inner: 3.0,
        check_box: 24.0,
        mark: 9.0,
        grip_len: 8.0,
        shadow_size: Vec2::splat(6.0),
        shadow_rel_offset: Vec2::ZERO,
        ..Default::default()
    }
}

const NORMS_FRAME: (f32, f32) = (-0.7, 0.7);
const NORMS_TRACK: (f32, f32) = (0.0, -0.7);
const NORMS_SUNK: (f32, f32) = (-0.7, 0.0);
const NORMS_RAISED: (f32, f32) = (0.0, 0.7);

pub struct DrawHandle<'a, DS: DrawSharedImpl> {
    draw: DrawIface<'a, DS>,
    ev: &'a mut EventState,
    w: &'a mut dim::Window<DS::Draw>,
    cols: &'a ColorsLinear,
}

impl<DS: DrawSharedImpl> Theme<DS> for ShadedTheme
where
    DS::Draw: DrawRoundedImpl + DrawShadedImpl,
{
    type Window = dim::Window<DS::Draw>;
    type Draw<'a> = DrawHandle<'a, DS>;

    fn config(&self) -> std::borrow::Cow<Config> {
        <SimpleTheme as Theme<DS>>::config(&self.base)
    }

    fn apply_config(&mut self, config: &Config) -> Action {
        <SimpleTheme as Theme<DS>>::apply_config(&mut self.base, config)
    }

    fn init(&mut self, shared: &mut SharedState<DS>) {
        <SimpleTheme as Theme<DS>>::init(&mut self.base, shared)
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        let fonts = self.base.fonts.as_ref().unwrap().clone();
        dim::Window::new(&dimensions(), &self.base.config, dpi_factor, fonts)
    }

    fn update_window(&self, w: &mut Self::Window, dpi_factor: f32) {
        w.update(&dimensions(), &self.base.config, dpi_factor);
    }

    fn draw<'a>(
        &'a self,
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        w: &'a mut Self::Window,
    ) -> Self::Draw<'a> {
        w.anim.update();

        DrawHandle {
            draw,
            ev,
            w,
            cols: &self.base.cols,
        }
    }

    fn draw_upcast<'a>(
        draw: DrawIface<'a, DS>,
        ev: &'a mut EventState,
        w: &'a mut Self::Window,
        cols: &'a ColorsLinear,
    ) -> Self::Draw<'a> {
        DrawHandle { draw, ev, w, cols }
    }

    fn clear_color(&self) -> Rgba {
        <SimpleTheme as Theme<DS>>::clear_color(&self.base)
    }
}

impl ThemeControl for ShadedTheme {
    fn set_font_size(&mut self, pt_size: f32) -> Action {
        self.base.set_font_size(pt_size)
    }

    fn active_scheme(&self) -> &str {
        self.base.active_scheme()
    }

    fn list_schemes(&self) -> Vec<&str> {
        self.base.list_schemes()
    }

    fn get_scheme(&self, name: &str) -> Option<&kas::theme::ColorsSrgb> {
        self.base.get_scheme(name)
    }

    fn get_colors(&self) -> &ColorsLinear {
        self.base.get_colors()
    }

    fn set_colors(&mut self, name: String, cols: ColorsLinear) -> Action {
        self.base.set_colors(name, cols)
    }
}

impl<'a, DS: DrawSharedImpl> DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl + DrawShadedImpl,
{
    // Type-cast to flat_theme's DrawHandle. Should be equivalent to transmute.
    fn as_flat<'b, 'c>(&'b mut self) -> <FlatTheme as Theme<DS>>::Draw<'c>
    where
        'a: 'c,
        'b: 'c,
    {
        FlatTheme::draw_upcast(self.draw.re(), self.ev, self.w, self.cols)
    }

    /// Draw an edit box with optional navigation highlight.
    /// Return the inner rect.
    fn draw_edit_box(&mut self, outer: Rect, bg_col: Rgba, nav_focus: bool) -> Quad {
        let outer = Quad::conv(outer);
        let inner = outer.shrink(self.w.dims.frame as f32);
        #[cfg(debug_assertions)]
        {
            if !inner.a.lt(inner.b) {
                log::warn!("draw_edit_box: frame too small: {outer:?}");
            }
        }

        let outer_col = self.cols.background;
        let inner_col = if nav_focus { self.cols.accent_soft } else { outer_col };
        self.draw
            .shaded_square_frame(outer, inner, NORMS_SUNK, outer_col, inner_col);

        self.draw.rect(inner, bg_col);
        inner
    }

    /// Draw a grip (for slider, scroll bar)
    fn draw_grip(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::conv(rect);
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.accent_soft_state(state);
        self.draw
            .shaded_round_frame(outer, inner, NORMS_RAISED, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(thickness / 4.0);
            self.draw.rounded_frame(outer, inner, 0.6, col);
        }
    }
}

#[kas::extends(ThemeDraw, base=self.as_flat())]
impl<'a, DS: DrawSharedImpl> ThemeDraw for DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl + DrawShadedImpl,
{
    fn components(&mut self) -> (&dyn ThemeSize, &mut dyn Draw, &mut EventState) {
        (self.w, &mut self.draw, self.ev)
    }

    fn new_pass<'b>(
        &mut self,
        inner_rect: Rect,
        offset: Offset,
        class: PassType,
        f: Box<dyn FnOnce(&mut dyn ThemeDraw) + 'b>,
    ) {
        let mut shadow = Default::default();
        let mut outer_rect = inner_rect;
        if class == PassType::Overlay {
            shadow = Quad::conv(inner_rect);
            shadow.a += self.w.dims.shadow_a;
            shadow.b += self.w.dims.shadow_b;
            let a = Coord::conv_floor(shadow.a);
            let b = Coord::conv_ceil(shadow.b);
            outer_rect = Rect::new(a, (b - a).cast());
        }
        let mut draw = self.draw.new_pass(outer_rect, offset, class);

        if class == PassType::Overlay {
            shadow += offset.cast();
            let inner = Quad::conv(inner_rect + offset);
            draw.rounded_frame_2col(shadow, inner, Rgba::BLACK, Rgba::TRANSPARENT);
        }

        let mut handle = DrawHandle {
            draw,
            ev: self.ev,
            w: self.w,
            cols: self.cols,
        };
        f(&mut handle);
    }

    fn frame(&mut self, id: &Id, rect: Rect, style: FrameStyle, bg: Background) {
        match style {
            FrameStyle::Frame => {
                let outer = Quad::conv(rect);
                let inner = outer.shrink(self.w.dims.frame as f32);
                let col = self.cols.background;
                self.draw.shaded_round_frame(outer, inner, NORMS_FRAME, col);
            }
            FrameStyle::Popup => {
                let outer = Quad::conv(rect);
                self.draw.rect(outer, self.cols.background);
            }
            FrameStyle::MenuEntry => {
                let state = InputState::new_all(self.ev, id);
                if let Some(col) = self.cols.menu_entry(state) {
                    let outer = Quad::conv(rect);
                    self.draw.rect(outer, col);
                }
            }
            FrameStyle::Button => {
                let state = InputState::new_all(self.ev, id);
                let outer = Quad::conv(rect);
                let frame = self.w.dims.button_frame as f32 + self.w.dims.button_inner as f32;
                let inner = outer.shrink(frame);
                let col_bg = self.cols.from_bg(bg, state, true);

                self.draw
                    .shaded_round_frame(outer, inner, NORMS_RAISED, col_bg);
                self.draw.rect(inner, col_bg);

                if let Some(col) = self.cols.nav_region(state) {
                    let outer = outer.shrink(self.w.dims.m_inner as f32);
                    self.draw.rounded_frame(outer, inner, 0.6, col);
                }
            }
            FrameStyle::EditBox => {
                let state = InputState::new_except_depress(self.ev, id);
                let bg_col = self.cols.from_edit_bg(bg, state);
                self.draw_edit_box(rect, bg_col, state.nav_focus());
            }
            FrameStyle::Window => {
                let outer = Quad::conv(rect);
                let inner = outer.shrink(self.w.dims.frame_window as f32);
                let col = self.cols.background;
                self.draw
                    .shaded_round_frame(outer, inner, NORMS_RAISED, col);
                self.draw.rect(inner, col);
            }
            style => self.as_flat().frame(id, rect, style, bg),
        }
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::conv(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, NORMS_TRACK, col);
    }

    fn check_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
        let state = InputState::new_all(self.ev, id);
        let bg_col = self.cols.from_edit_bg(Default::default(), state);
        let inner = self.draw_edit_box(rect, bg_col, state.nav_focus());

        self.as_flat()
            .check_mark(inner, state, checked, last_change);
    }

    fn radio_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
        let state = InputState::new_all(self.ev, id);
        let anim_fade = 1.0 - self.w.anim.fade_bool(self.draw.draw, checked, last_change);

        let bg_col = self.cols.from_edit_bg(Default::default(), state);

        let inner = self
            .draw_edit_box(rect, bg_col, state.nav_focus())
            .shrink(self.w.dims.m_inner as f32);

        if anim_fade < 1.0 {
            let v = inner.size() * (anim_fade / 2.0);
            let inner = Quad::from_coords(inner.a + v, inner.b - v);
            let col = self.cols.check_mark_state(state);
            self.draw.shaded_circle(inner, NORMS_RAISED, col);
        }
    }

    fn scroll_bar(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, _: Direction) {
        // track
        let outer = Quad::conv(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, NORMS_TRACK, col);

        // grip
        let state = InputState::new2(self.ev, id, id2);
        self.draw_grip(h_rect, state);
    }

    fn slider(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, dir: Direction) {
        // track
        let mut outer = Quad::conv(rect);
        outer = match dir.is_horizontal() {
            true => outer.shrink_vec(Vec2(0.0, outer.size().1 * (3.0 / 8.0))),
            false => outer.shrink_vec(Vec2(outer.size().0 * (3.0 / 8.0), 0.0)),
        };
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, NORMS_TRACK, col);

        // grip
        let state = InputState::new2(self.ev, id, id2);
        self.draw_grip(h_rect, state);
    }

    fn progress_bar(&mut self, _: &Id, rect: Rect, dir: Direction, value: f32) {
        let mut outer = Quad::conv(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.frame;
        self.draw.shaded_round_frame(outer, inner, NORMS_TRACK, col);

        if dir.is_horizontal() {
            outer.b.0 = outer.a.0 + value * (outer.b.0 - outer.a.0);
        } else {
            outer.b.1 = outer.a.1 + value * (outer.b.1 - outer.a.1);
        }
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.accent_soft;
        self.draw
            .shaded_round_frame(outer, inner, NORMS_RAISED, col);
    }
}
