// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Flat theme

use std::cell::RefCell;
use std::f32;
use std::ops::Range;
use std::time::Instant;

use super::SimpleTheme;
use crate::Id;
use crate::cast::traits::*;
use crate::config::{Config, WindowConfig};
use crate::dir::{Direction, Directional};
use crate::draw::{color::Rgba, *};
use crate::event::EventState;
use crate::geom::*;
use crate::text::TextDisplay;
use crate::theme::dimensions as dim;
use crate::theme::{Background, FrameStyle, MarkStyle};
use crate::theme::{ColorsLinear, InputState, Theme};
use crate::theme::{ThemeDraw, ThemeSize};

// Used to ensure a rectangular background is inside a circular corner.
// Also the maximum inner radius of circular borders to overlap with this rect.
const BG_SHRINK_FACTOR: f32 = 1.0 - std::f32::consts::FRAC_1_SQRT_2;

// Shadow enlargement on mouse over
const SHADOW_MOUSE_OVER: f32 = 1.1;
// Shadow enlargement for pop-ups
const SHADOW_POPUP: f32 = 1.2;

#[derive(PartialEq, Eq)]
enum ShadowStyle {
    None,
    Normal,
    MouseOver,
}

/// A theme with flat (unshaded) rendering
///
/// This is a fully functional theme using only the basic drawing primitives
/// available by default.
#[derive(Clone, Debug)]
pub struct FlatTheme {
    base: SimpleTheme,
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
        let base = SimpleTheme::new();
        FlatTheme { base }
    }
}

fn dimensions() -> dim::Parameters {
    dim::Parameters {
        // NOTE: visual thickness is (button_frame * scale_factor).round() * (1 - BG_SHRINK_FACTOR)
        button_frame: 2.4,
        button_inner: 0.0,
        slider_size: Vec2(24.0, 18.0),
        shadow_size: Vec2(4.0, 4.0),
        shadow_rel_offset: Vec2(0.2, 0.3),
        ..Default::default()
    }
}

pub struct DrawHandle<'a, DS: DrawSharedImpl> {
    pub(crate) draw: DrawIface<'a, DS>,
    pub(crate) ev: &'a mut EventState,
    pub(crate) w: &'a mut dim::Window<DS::Draw>,
    pub(crate) cols: &'a ColorsLinear,
}

impl<DS: DrawSharedImpl> Theme<DS> for FlatTheme
where
    DS::Draw: DrawRoundedImpl,
{
    type Window = dim::Window<DS::Draw>;
    type Draw<'a> = DrawHandle<'a, DS>;

    fn init(&mut self, config: &RefCell<Config>) {
        <SimpleTheme as Theme<DS>>::init(&mut self.base, config)
    }

    fn new_window(&mut self, config: &WindowConfig) -> Self::Window {
        self.base.cols = config.theme().get_active_scheme().into();
        dim::Window::new(&dimensions(), config)
    }

    fn update_window(&mut self, w: &mut Self::Window, config: &WindowConfig) {
        self.base.cols = config.theme().get_active_scheme().into();
        w.update(&dimensions(), config);
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
        self.base.cols.background
    }
}

impl<'a, DS: DrawSharedImpl> DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    // Type-cast to simple_theme's DrawHandle. Should be equivalent to transmute.
    fn as_simple<'b, 'c>(&'b mut self) -> super::simple_theme::DrawHandle<'c, DS>
    where
        'a: 'c,
        'b: 'c,
    {
        super::simple_theme::DrawHandle {
            draw: self.draw.re(),
            ev: self.ev,
            w: self.w,
            cols: self.cols,
        }
    }

    fn button_frame(
        &mut self,
        outer: Quad,
        inner: Quad,
        col_frame: Rgba,
        col_bg: Rgba,
        shadow: ShadowStyle,
    ) -> Quad {
        #[cfg(debug_assertions)]
        {
            if !(inner.a < inner.b) {
                log::warn!("button_frame: frame too small: {outer:?}");
            }
        }

        if shadow != ShadowStyle::None {
            let (mut a, mut b) = (self.w.dims.shadow_a, self.w.dims.shadow_b);
            if shadow == ShadowStyle::MouseOver {
                a *= SHADOW_MOUSE_OVER;
                b *= SHADOW_MOUSE_OVER;
            }
            let shadow_outer = Quad::from_coords(a + inner.a, b + inner.b);
            let col1 = if self.cols.is_dark { col_frame } else { Rgba::BLACK };
            let mut col2 = col1;
            col2.a = 0.0;
            self.draw
                .rounded_frame_2col(shadow_outer, inner, col1, col2);
        }

        let bgr = outer.shrink(self.w.dims.button_frame as f32 * BG_SHRINK_FACTOR);
        self.draw.rect(bgr, col_bg);

        self.draw
            .rounded_frame(outer, inner, BG_SHRINK_FACTOR, col_frame);
        inner
    }

    pub fn edit_box(&mut self, id: &Id, outer: Quad, bg: Background) {
        let state = InputState::new_except_depress(self.ev, id);
        let col_bg = self.cols.from_edit_bg(bg, state);
        if col_bg != self.cols.background {
            let inner = outer.shrink(self.w.dims.button_frame as f32 * BG_SHRINK_FACTOR);
            self.draw.rect(inner, col_bg);
        }

        let inner = outer.shrink(self.w.dims.button_frame as f32);
        self.draw
            .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);

        if !state.disabled() && !self.cols.is_dark && (state.nav_focus() || state.under_mouse()) {
            let r = 0.5 * self.w.dims.button_frame as f32;
            let y = outer.b.1 - r;
            let a = Vec2(outer.a.0 + r, y);
            let b = Vec2(outer.b.0 - r, y);
            let col = if state.nav_focus() {
                self.cols.nav_focus
            } else {
                self.cols.text
            };

            const F: f32 = 0.6;
            let (sa, sb) = (self.w.dims.shadow_a * F, self.w.dims.shadow_b * F);
            let outer = Quad::from_coords(a + sa, b + sb);
            let inner = Quad::from_coords(a, b);
            let col1 = if self.cols.is_dark { col } else { Rgba::BLACK };
            let mut col2 = col1;
            col2.a = 0.0;
            self.draw.rounded_frame_2col(outer, inner, col1, col2);

            self.draw.rounded_line(a, b, r, col);
        }
    }

    pub fn check_mark(
        &mut self,
        inner: Quad,
        state: InputState,
        checked: bool,
        last_change: Option<Instant>,
    ) {
        let anim_fade = 1.0 - self.w.anim.fade_bool(self.draw.draw, checked, last_change);
        if anim_fade < 1.0 {
            let inner = inner.shrink(self.w.dims.m_inner as f32);
            let v = inner.size() * (anim_fade / 2.0);
            let inner = Quad::from_coords(inner.a + v, inner.b - v);
            let col = self.cols.check_mark_state(state);
            let f = self.w.dims.mark_line;
            if inner.size().min_comp() >= 2.0 * f {
                let inner = inner.shrink(f);
                let size = inner.size();
                let vstep = size.1 * 0.125;
                let a = Vec2(inner.a.0, inner.b.1 - 3.0 * vstep);
                let b = Vec2(inner.a.0 + size.0 * 0.25, inner.b.1 - vstep);
                let c = Vec2(inner.b.0, inner.a.1 + vstep);
                self.draw.rounded_line(a, b, f, col);
                self.draw.rounded_line(b, c, f, col);
            } else {
                self.draw.rect(inner, col);
            }
        }
    }
}

#[kas::extends(ThemeDraw, base=self.as_simple())]
impl<'a, DS: DrawSharedImpl> ThemeDraw for DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    fn components(&mut self) -> (&dyn ThemeSize, &mut dyn Draw, &mut EventState) {
        (self.w, &mut self.draw, self.ev)
    }

    fn colors(&self) -> &ColorsLinear {
        &self.cols
    }

    fn draw_rounded(&mut self) -> Option<&mut dyn DrawRounded> {
        Some(&mut self.draw)
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
            shadow.a += self.w.dims.shadow_a * SHADOW_POPUP;
            shadow.b += self.w.dims.shadow_b * SHADOW_POPUP;
            let a = Coord::conv_floor(shadow.a);
            let b = Coord::conv_ceil(shadow.b);
            outer_rect = Rect::new(a, (b - a).cast());
        }
        let mut draw = self.draw.new_pass(outer_rect, offset, class);

        if class == PassType::Overlay {
            shadow += offset.cast();
            let inner = Quad::conv(inner_rect + offset).shrink(self.w.dims.menu_frame as f32);
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
        let outer = Quad::conv(rect);
        match style {
            FrameStyle::None => {
                let state = InputState::new_except_depress(self.ev, id);
                let col = self.cols.from_bg(bg, state, false);
                self.draw.rect(outer, col);
            }
            FrameStyle::Frame => {
                let inner = outer.shrink(self.w.dims.frame as f32);
                self.draw
                    .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
            }
            FrameStyle::Window => {
                let inner = outer.shrink(self.w.dims.frame_window as f32);
                self.draw
                    .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
            }
            FrameStyle::Popup => {
                // We cheat here by using zero-sized popup-frame, but assuming that contents are
                // all a MenuEntry, and drawing into this space. This might look wrong if other
                // widgets are used in the popup.
                let size = self.w.dims.menu_frame as f32;
                let inner = outer.shrink(size);
                self.draw
                    .rounded_frame(outer, inner, BG_SHRINK_FACTOR, self.cols.frame);
                let inner = outer.shrink(size * BG_SHRINK_FACTOR);
                self.draw.rect(inner, self.cols.background);
            }
            FrameStyle::MenuEntry => {
                let state = InputState::new_all(self.ev, id);
                if let Some(col) = self.cols.menu_entry(state) {
                    let size = self.w.dims.menu_frame as f32;
                    let inner = outer.shrink(size);
                    self.draw.rounded_frame(outer, inner, BG_SHRINK_FACTOR, col);
                    let inner = outer.shrink(size * BG_SHRINK_FACTOR);
                    self.draw.rect(inner, col);
                }
            }
            FrameStyle::NavFocus => {
                let state = InputState::new_all(self.ev, id);
                if let Some(col) = self.cols.nav_region(state) {
                    let inner = outer.shrink(self.w.dims.m_inner as f32);
                    self.draw.rounded_frame(outer, inner, 0.0, col);
                }
            }
            FrameStyle::Button => {
                let state = InputState::new_all(self.ev, id);
                let outer = Quad::conv(rect);
                let inner = outer.shrink(self.w.dims.button_frame as f32);

                let col_bg = self.cols.from_bg(bg, state, false);
                let col_frame = self.cols.nav_region(state).unwrap_or(self.cols.frame);

                let shadow = match () {
                    () if (self.cols.is_dark || state.disabled() || state.depress()) => {
                        ShadowStyle::None
                    }
                    () if state.under_mouse() => ShadowStyle::MouseOver,
                    _ => ShadowStyle::Normal,
                };

                self.button_frame(outer, inner, col_frame, col_bg, shadow);
            }
            FrameStyle::Tab => {
                let state = InputState::new_all(self.ev, id);
                let outer = Quad::conv(rect);
                let w = self.w.dims.button_frame as f32;
                let inner = Quad::from_coords(outer.a + w, outer.b - Vec2(w, 0.0));

                let col_bg = self.cols.from_bg(bg, state, false);
                let col_frame = self.cols.nav_region(state).unwrap_or(self.cols.frame);

                self.button_frame(outer, inner, col_frame, col_bg, ShadowStyle::None);
            }
            FrameStyle::EditBox => self.edit_box(id, outer, bg),
        }
    }

    fn check_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
        let state = InputState::new_all(self.ev, id);
        let outer = Quad::conv(rect);
        let inner = outer.shrink(self.w.dims.button_frame as f32);

        let col_frame = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        let col_bg = self.cols.from_edit_bg(Default::default(), state);

        let shadow = match () {
            () if (self.cols.is_dark || state.disabled() || state.depress()) => ShadowStyle::None,
            () if state.under_mouse() => ShadowStyle::MouseOver,
            _ => ShadowStyle::Normal,
        };

        let inner = self.button_frame(outer, inner, col_frame, col_bg, shadow);

        self.check_mark(inner, state, checked, last_change);
    }

    fn radio_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
        let anim_fade = 1.0 - self.w.anim.fade_bool(self.draw.draw, checked, last_change);

        let state = InputState::new_all(self.ev, id);
        let outer = Quad::conv(rect);
        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);

        if !(self.cols.is_dark || state.disabled() || state.depress()) {
            let (mut a, mut b) = (self.w.dims.shadow_a, self.w.dims.shadow_b);
            let mut mult = 0.65;
            if state.under_mouse() {
                mult *= SHADOW_MOUSE_OVER;
            }
            a *= mult;
            b *= mult;
            let shadow_outer = Quad::from_coords(a + outer.a, b + outer.b);
            let col1 = if self.cols.is_dark { col } else { Rgba::BLACK };
            let mut col2 = col1;
            col2.a = 0.0;
            self.draw.circle_2col(shadow_outer, col1, col2);
        }

        let col_bg = self.cols.from_edit_bg(Default::default(), state);
        self.draw.circle(outer, 0.0, col_bg);

        const F: f32 = 2.0 * (1.0 - BG_SHRINK_FACTOR); // match check box frame
        let r = 1.0 - F * self.w.dims.button_frame as f32 / rect.size.0 as f32;
        self.draw.circle(outer, r, col);

        if anim_fade < 1.0 {
            let r = self.w.dims.button_frame + self.w.dims.m_inner as i32;
            let inner = outer.shrink(r as f32);
            let v = inner.size() * (anim_fade / 2.0);
            let inner = Quad::from_coords(inner.a + v, inner.b - v);
            let col = self.cols.check_mark_state(state);
            self.draw.circle(inner, 0.0, col);
        }
    }

    fn scroll_bar(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, _: Direction) {
        // track
        let outer = Quad::conv(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let mut col = self.cols.frame;
        col.a = 0.5; // HACK
        self.draw.rounded_frame(outer, inner, 0.0, col);

        // grip
        let outer = Quad::conv(h_rect);
        let r = outer.size().min_comp() * 0.125;
        let outer = outer.shrink(r);
        let inner = outer.shrink(3.0 * r);
        let state = InputState::new2(self.ev, id, id2);
        let col = self.cols.accent_soft_state(state);
        self.draw.rounded_frame(outer, inner, 0.0, col);
    }

    fn slider(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, dir: Direction) {
        let state = InputState::new2(self.ev, id, id2);

        // track
        let mut outer = Quad::conv(rect);
        let mid = Vec2::conv(h_rect.pos + h_rect.size / 2);
        let (mut first, mut second);
        if dir.is_horizontal() {
            outer = outer.shrink_vec(Vec2(0.0, outer.size().1 * (1.0 / 3.0)));
            first = Quad::from_coords(outer.a, Vec2(mid.0, outer.b.1));
            second = Quad::from_coords(Vec2(mid.0, outer.a.1), outer.b);
        } else {
            outer = outer.shrink_vec(Vec2(outer.size().0 * (1.0 / 3.0), 0.0));
            first = Quad::from_coords(outer.a, Vec2(outer.b.0, mid.1));
            second = Quad::from_coords(Vec2(outer.a.0, mid.1), outer.b);
        };
        if dir.is_reversed() {
            std::mem::swap(&mut first, &mut second);
        }

        let inner = first.shrink(first.size().min_comp() / 2.0);
        self.draw.rounded_frame(first, inner, 0.0, self.cols.accent);
        let inner = second.shrink(second.size().min_comp() / 2.0);
        self.draw
            .rounded_frame(second, inner, 1.0 / 3.0, self.cols.frame);

        // grip; force it to be square
        let size = Size::splat(h_rect.size.0.min(h_rect.size.1));
        let offset = Offset::conv((h_rect.size - size) / 2);
        let outer = Quad::conv(Rect::new(h_rect.pos + offset, size));

        let col = if state.nav_focus() && !state.disabled() {
            self.cols.accent_soft
        } else {
            self.cols.background
        };
        let col = ColorsLinear::adjust_for_state(col, state);

        if !self.cols.is_dark && !state.contains(InputState::DISABLED | InputState::DEPRESS) {
            let (mut a, mut b) = (self.w.dims.shadow_a, self.w.dims.shadow_b);
            let mut mult = 0.6;
            if state.under_mouse() {
                mult *= SHADOW_MOUSE_OVER;
            }
            a *= mult;
            b *= mult;
            let shadow_outer = Quad::from_coords(a + outer.a, b + outer.b);
            let col1 = if self.cols.is_dark { col } else { Rgba::BLACK };
            let mut col2 = col1;
            col2.a = 0.0;
            self.draw.circle_2col(shadow_outer, col1, col2);
        }

        self.draw.circle(outer, 0.0, col);
        let col = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        self.draw.circle(outer, 14.0 / 16.0, col);
    }

    fn progress_bar(&mut self, _: &Id, rect: Rect, dir: Direction, value: f32) {
        let mut outer = Quad::conv(rect);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        self.draw.rounded_frame(outer, inner, 0.75, self.cols.frame);

        if dir.is_horizontal() {
            outer.b.0 = outer.a.0 + value * (outer.b.0 - outer.a.0);
        } else {
            outer.b.1 = outer.a.1 + value * (outer.b.1 - outer.a.1);
        }
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        self.draw.rounded_frame(outer, inner, 0.0, self.cols.accent);
    }
}
