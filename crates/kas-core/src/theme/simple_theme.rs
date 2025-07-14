// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple theme

use std::cell::RefCell;
use std::f32;
use std::ops::Range;
use std::time::Instant;

use crate::Id;
use crate::cast::traits::*;
use crate::config::{Config, WindowConfig};
use crate::dir::{Direction, Directional};
use crate::draw::{color::Rgba, *};
use crate::event::EventState;
use crate::geom::*;
use crate::text::{Effect, TextDisplay};
use crate::theme::dimensions as dim;
use crate::theme::{Background, FrameStyle, MarkStyle, TextClass};
use crate::theme::{ColorsLinear, InputState, Theme};
use crate::theme::{SelectionStyle, ThemeDraw, ThemeSize};

use super::ColorsSrgb;

/// A simple theme
///
/// This theme is functional, but not pretty. It is intended as a template for
/// other themes.
#[derive(Clone, Debug)]
pub struct SimpleTheme {
    pub cols: ColorsLinear,
    dims: dim::Parameters,
}

impl Default for SimpleTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleTheme {
    /// Construct
    #[inline]
    pub fn new() -> Self {
        SimpleTheme {
            cols: ColorsSrgb::LIGHT.into(), // value is unimportant
            dims: Default::default(),
        }
    }
}

pub struct DrawHandle<'a, DS: DrawSharedImpl> {
    pub(crate) draw: DrawIface<'a, DS>,
    pub(crate) ev: &'a mut EventState,
    pub(crate) w: &'a mut dim::Window<DS::Draw>,
    pub(crate) cols: &'a ColorsLinear,
}

impl<DS: DrawSharedImpl> Theme<DS> for SimpleTheme
where
    DS::Draw: DrawRoundedImpl,
{
    type Window = dim::Window<DS::Draw>;
    type Draw<'a> = DrawHandle<'a, DS>;

    fn init(&mut self, _: &RefCell<Config>) {}

    fn new_window(&mut self, config: &WindowConfig) -> Self::Window {
        self.cols = config.theme().get_active_scheme().into();
        dim::Window::new(&self.dims, config)
    }

    fn update_window(&mut self, w: &mut Self::Window, config: &WindowConfig) {
        self.cols = config.theme().get_active_scheme().into();
        w.update(&self.dims, config);
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
            cols: &self.cols,
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
        self.cols.background
    }
}

impl<'a, DS: DrawSharedImpl> DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
{
    pub fn button_frame(
        &mut self,
        outer: Quad,
        col_frame: Rgba,
        col_bg: Rgba,
        _: InputState,
    ) -> Quad {
        let inner = outer.shrink(self.w.dims.button_frame as f32);
        #[cfg(debug_assertions)]
        {
            if !inner.a.lt(inner.b) {
                log::warn!("button_frame: frame too small: {outer:?}");
            }
        }

        let bgr = outer.shrink(self.w.dims.button_frame as f32);
        self.draw.rect(bgr, col_bg);

        self.draw.frame(outer, inner, col_frame);
        inner
    }

    pub fn edit_box(&mut self, id: &Id, outer: Quad, bg: Background) {
        let state = InputState::new_except_depress(self.ev, id);
        let col_bg = self.cols.from_edit_bg(bg, state);
        if col_bg != self.cols.background {
            let inner = outer.shrink(self.w.dims.button_frame as f32);
            self.draw.rect(inner, col_bg);
        }

        let inner = outer.shrink(self.w.dims.button_frame as f32);
        self.draw.frame(outer, inner, self.cols.frame);

        if !state.disabled() && !self.cols.is_dark && (state.nav_focus() || state.hover()) {
            let mut line = outer;
            line.a.1 = line.b.1 - self.w.dims.button_frame as f32;
            let col = if state.nav_focus() {
                self.cols.nav_focus
            } else {
                self.cols.text
            };
            self.draw.rect(line, col);
        }
    }

    fn draw_mark(&mut self, rect: Rect, style: MarkStyle, col: Rgba) {
        match style {
            MarkStyle::Chevron(dir) => {
                let size = match dir.is_horizontal() {
                    true => Size(self.w.dims.mark / 2, self.w.dims.mark),
                    false => Size(self.w.dims.mark, self.w.dims.mark / 2),
                };
                let offset = Offset::conv((rect.size - size) / 2);
                let q = Quad::conv(Rect::new(rect.pos + offset, size));

                let (p1, p2, p3);
                if dir.is_horizontal() {
                    let (mut x1, mut x2) = (q.a.0, q.b.0);
                    if dir.is_reversed() {
                        std::mem::swap(&mut x1, &mut x2);
                    }
                    p1 = Vec2(x1, q.a.1);
                    p2 = Vec2(x2, 0.5 * (q.a.1 + q.b.1));
                    p3 = Vec2(x1, q.b.1);
                } else {
                    let (mut y1, mut y2) = (q.a.1, q.b.1);
                    if dir.is_reversed() {
                        std::mem::swap(&mut y1, &mut y2);
                    }
                    p1 = Vec2(q.a.0, y1);
                    p2 = Vec2(0.5 * (q.a.0 + q.b.0), y2);
                    p3 = Vec2(q.b.0, y1);
                };

                let f = 0.5 * self.w.dims.mark_line;
                self.draw.rounded_line(p1, p2, f, col);
                self.draw.rounded_line(p2, p3, f, col);
            }
            MarkStyle::X => {
                let size = Size::splat(self.w.dims.mark);
                let offset = Offset::conv((rect.size - size) / 2);
                let q = Quad::conv(Rect::new(rect.pos + offset, size));

                let f = 0.5 * self.w.dims.mark_line;
                self.draw.rounded_line(q.a, q.b, f, col);
                let c = Vec2(q.a.0, q.b.1);
                let d = Vec2(q.b.0, q.a.1);
                self.draw.rounded_line(c, d, f, col);
            }
        }
    }
}

impl<'a, DS: DrawSharedImpl> ThemeDraw for DrawHandle<'a, DS>
where
    DS::Draw: DrawRoundedImpl,
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
        let draw = self.draw.new_pass(inner_rect, offset, class);
        let mut handle = DrawHandle {
            draw,
            ev: self.ev,
            w: self.w,
            cols: self.cols,
        };
        f(&mut handle);
    }

    fn get_clip_rect(&mut self) -> Rect {
        self.draw.get_clip_rect()
    }

    fn event_state_overlay(&mut self) {
        if let Some((coord, used)) = self.ev.mouse_pin() {
            // let r = self.w.dims.scale * 6.0;
            // let inner = if used { 0.0 } else { 0.6 };
            let (r, inner) = match used {
                false => (self.w.dims.scale * 3.6, 0.0),
                true => (self.w.dims.scale * 6.0, 0.6),
            };
            let c = self.cols.accent;
            self.draw
                .circle(Quad::from_center(coord.cast_approx(), r), inner, c);
        }
    }

    fn frame(&mut self, id: &Id, rect: Rect, style: FrameStyle, bg: Background) {
        let outer = Quad::conv(rect);
        match style {
            FrameStyle::None => {
                let state = InputState::new_except_depress(self.ev, id);
                let col = self.cols.from_bg(bg, state, false);
                self.draw.rect(outer, col);
            }
            FrameStyle::Frame | FrameStyle::Window => {
                let inner = outer.shrink(self.w.dims.frame as f32);
                self.draw.frame(outer, inner, self.cols.frame);
            }
            FrameStyle::Popup => {
                // We cheat here by using zero-sized popup-frame, but assuming that contents are
                // all a MenuEntry, and drawing into this space. This might look wrong if other
                // widgets are used in the popup.
                let size = self.w.dims.menu_frame as f32;
                let inner = outer.shrink(size);
                self.draw.frame(outer, inner, self.cols.frame);
                self.draw.rect(inner, self.cols.background);
            }
            FrameStyle::MenuEntry => {
                let state = InputState::new_all(self.ev, id);
                if let Some(col) = self.cols.menu_entry(state) {
                    self.draw.rect(outer, col);
                }
            }
            FrameStyle::NavFocus => {
                let state = InputState::new_all(self.ev, id);
                if let Some(col) = self.cols.nav_region(state) {
                    let inner = outer.shrink(self.w.dims.m_inner as f32);
                    self.draw.frame(outer, inner, col);
                }
            }
            FrameStyle::Button | FrameStyle::Tab => {
                let state = InputState::new_all(self.ev, id);
                let outer = Quad::conv(rect);

                let col_bg = self.cols.from_bg(bg, state, false);
                let col_frame = self.cols.nav_region(state).unwrap_or(self.cols.frame);
                self.button_frame(outer, col_frame, col_bg, state);
            }
            FrameStyle::EditBox => self.edit_box(id, outer, bg),
        }
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::conv(rect);
        self.draw.rect(outer, self.cols.frame);
    }

    fn selection(&mut self, rect: Rect, style: SelectionStyle) {
        let inner = Quad::conv(rect);
        match style {
            SelectionStyle::Highlight => {
                self.draw.rect(inner, self.cols.text_sel_bg);
            }
            SelectionStyle::Frame => {
                let outer = inner.grow(self.w.dims.m_inner.into());
                // TODO: this should use its own colour and a stippled pattern
                let col = self.cols.accent;
                self.draw.frame(outer, inner, col);
            }
            SelectionStyle::Both => {
                let outer = inner.grow(self.w.dims.m_inner.into());
                self.draw.rect(outer, self.cols.accent);
                self.draw.rect(inner, self.cols.text_sel_bg);
            }
        }
    }

    fn text(&mut self, id: &Id, rect: Rect, text: &TextDisplay, _: TextClass) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            self.cols.text
        };
        self.draw.text(rect, text, col);
    }

    fn text_effects(
        &mut self,
        id: &Id,
        rect: Rect,
        text: &TextDisplay,
        effects: &[Effect<()>],
        class: TextClass,
    ) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            self.cols.text
        };
        if class.is_access_key() && !self.ev.show_access_labels() {
            self.draw.text(rect, text, col);
        } else {
            self.draw.text_effects(rect, text, col, effects);
        }
    }

    fn text_selected_range(
        &mut self,
        id: &Id,
        rect: Rect,
        text: &TextDisplay,
        range: Range<usize>,
        _: TextClass,
    ) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            self.cols.text
        };
        let sel_col = self.cols.text_over(self.cols.text_sel_bg);

        // Draw background:
        text.highlight_range(range.clone(), &mut |p1, p2| {
            let q = Quad::conv(rect);
            let p1 = Vec2::from(p1);
            let p2 = Vec2::from(p2);
            if let Some(quad) = Quad::from_coords(q.a + p1, q.a + p2).intersection(&q) {
                self.draw.rect(quad, self.cols.text_sel_bg);
            }
        });

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
        self.draw.text_effects_rgba(rect, text, &effects);
    }

    fn text_cursor(&mut self, id: &Id, rect: Rect, text: &TextDisplay, _: TextClass, byte: usize) {
        if self.ev.window_has_focus() && !self.w.anim.text_cursor(self.draw.draw, id, byte) {
            return;
        }

        let width = self.w.dims.mark_line;
        let pos = Vec2::conv(rect.pos);
        let p10max = pos.0 + f32::conv(rect.size.0) - width;

        let mut col = self.cols.nav_focus;
        for cursor in text.text_glyph_pos(byte).rev() {
            let mut p1 = pos + Vec2::from(cursor.pos);
            p1.0 = p1.0.min(p10max);
            let mut p2 = p1;
            p1.1 -= cursor.ascent;
            p2.1 -= cursor.descent;
            p2.0 += width;
            let quad = Quad::from_coords(p1, p2);
            self.draw.rect(quad, col);

            if cursor.embedding_level() > 0 {
                // Add a hat to indicate directionality.
                let height = width;
                let quad = if cursor.is_ltr() {
                    Quad::from_coords(Vec2(p2.0, p1.1), Vec2(p2.0 + width, p1.1 + height))
                } else {
                    Quad::from_coords(Vec2(p1.0 - width, p1.1), Vec2(p1.0, p1.1 + height))
                };
                self.draw.rect(quad, col);
            }
            // hack to make secondary marker grey:
            col = col.average();
        }
    }

    fn check_box(&mut self, id: &Id, rect: Rect, checked: bool, _: Option<Instant>) {
        let state = InputState::new_all(self.ev, id);
        let outer = Quad::conv(rect);

        let col_frame = self.cols.nav_region(state).unwrap_or(self.cols.frame);
        let col_bg = self.cols.from_edit_bg(Default::default(), state);
        let inner = self.button_frame(outer, col_frame, col_bg, state);

        if checked {
            let inner = inner.shrink(self.w.dims.m_inner as f32);
            let col = self.cols.check_mark_state(state);
            self.draw.rect(inner, col);
        }
    }

    fn radio_box(&mut self, id: &Id, rect: Rect, checked: bool, last_change: Option<Instant>) {
        self.check_box(id, rect, checked, last_change);
    }

    fn mark(&mut self, id: &Id, rect: Rect, style: MarkStyle) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            let is_depressed = self.ev.is_depressed(id);
            if self.ev.is_hovered(id) || is_depressed {
                self.draw.rect(rect.cast(), self.cols.accent_soft);
            }
            if is_depressed { self.cols.accent } else { self.cols.text }
        };

        self.draw_mark(rect, style, col);
    }

    fn scroll_bar(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, _: Direction) {
        let track = Quad::conv(rect);
        self.draw.rect(track, self.cols.frame);

        let grip = Quad::conv(h_rect);
        let state = InputState::new2(self.ev, id, id2);
        let col = self.cols.accent_soft_state(state);
        self.draw.rect(grip, col);
    }

    fn slider(&mut self, id: &Id, id2: &Id, rect: Rect, h_rect: Rect, _: Direction) {
        let track = Quad::conv(rect);
        self.draw.rect(track, self.cols.frame);

        let grip = Quad::conv(h_rect);
        let state = InputState::new2(self.ev, id, id2);
        let col = self.cols.accent_soft_state(state);
        self.draw.rect(grip, col);
    }

    fn progress_bar(&mut self, _: &Id, rect: Rect, dir: Direction, value: f32) {
        let mut outer = Quad::conv(rect);
        self.draw.rect(outer, self.cols.frame);

        if dir.is_horizontal() {
            outer.b.0 = outer.a.0 + value * (outer.b.0 - outer.a.0);
        } else {
            outer.b.1 = outer.a.1 + value * (outer.b.1 - outer.a.1);
        }
        self.draw.rect(outer, self.cols.accent);
    }

    fn image(&mut self, id: ImageId, rect: Rect) {
        self.draw.image(id, rect.cast());
    }
}
