// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Simple theme

use linear_map::LinearMap;
use std::f32;
use std::ops::Range;
use std::rc::Rc;
use std::time::Instant;

use crate::{dim, ColorsLinear, Config, InputState, Theme};
use kas::cast::traits::*;
use kas::dir::{Direction, Directional};
use kas::draw::{color::Rgba, *};
use kas::event::EventState;
use kas::geom::*;
use kas::text::{fonts, Effect, TextApi, TextDisplay};
use kas::theme::{Background, FrameStyle, MarkStyle, TextClass};
use kas::theme::{ThemeControl, ThemeDraw, ThemeSize};
use kas::{TkAction, WidgetId};

/// A simple theme
#[derive(Clone, Debug)]
pub struct SimpleTheme {
    pub(crate) config: Config,
    pub(crate) cols: ColorsLinear,
    dims: dim::Parameters,
    pub(crate) fonts: Option<Rc<LinearMap<TextClass, fonts::FontId>>>,
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
        let cols = ColorsLinear::default();
        SimpleTheme {
            config: Default::default(),
            cols,
            dims: Default::default(),
            fonts: None,
        }
    }

    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    #[inline]
    #[must_use]
    pub fn with_font_size(mut self, pt_size: f32) -> Self {
        self.config.set_font_size(pt_size);
        self
    }

    /// Set the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    #[inline]
    #[must_use]
    pub fn with_colours(mut self, name: &str) -> Self {
        if let Some(scheme) = self.config.get_color_scheme(name) {
            self.config.set_active_scheme(name);
            let _ = self.set_colors(scheme.into());
        }
        self
    }

    pub fn set_colors(&mut self, cols: ColorsLinear) -> TkAction {
        self.cols = cols;
        TkAction::REDRAW
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
    type Config = Config;
    type Window = dim::Window<DS::Draw>;

    type Draw<'a> = DrawHandle<'a, DS>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        std::borrow::Cow::Borrowed(&self.config)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        let mut action = self.config.apply_config(config);
        if let Some(scheme) = self.config.get_active_scheme() {
            action |= self.set_colors(scheme.into());
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
        dim::Window::new(&self.dims, &self.config, dpi_factor, fonts)
    }

    fn update_window(&self, w: &mut Self::Window, dpi_factor: f32) {
        w.update(&self.dims, &self.config, dpi_factor);
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

    fn clear_color(&self) -> Rgba {
        self.cols.background
    }
}

impl ThemeControl for SimpleTheme {
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
                self.config.set_active_scheme(name);
                return self.set_colors(scheme.into());
            }
        }
        TkAction::empty()
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

    pub fn edit_box(&mut self, id: &WidgetId, outer: Quad, bg: Background) {
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

    fn frame(&mut self, id: &WidgetId, rect: Rect, style: FrameStyle, bg: Background) {
        let outer = Quad::conv(rect);
        match style {
            FrameStyle::Frame => {
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
            FrameStyle::Button => {
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

    fn selection_box(&mut self, rect: Rect) {
        let inner = Quad::conv(rect);
        let outer = inner.grow(self.w.dims.m_inner.into());
        // TODO: this should use its own colour and a stippled pattern
        let col = self.cols.text_sel_bg;
        self.draw.frame(outer, inner, col);
    }

    fn text(&mut self, id: &WidgetId, rect: Rect, text: &TextDisplay, _: TextClass) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            self.cols.text
        };
        self.draw.text(rect, text, col);
    }

    fn text_effects(&mut self, id: &WidgetId, rect: Rect, text: &dyn TextApi, class: TextClass) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else {
            self.cols.text
        };
        if class.is_accel() && !self.ev.show_accel_labels() {
            self.draw.text(rect, text.display(), col);
        } else {
            self.draw
                .text_effects(rect, text.display(), col, text.effect_tokens());
        }
    }

    fn text_selected_range(
        &mut self,
        id: &WidgetId,
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
        let result = text.highlight_range(range.clone(), &mut |p1, p2| {
            let q = Quad::conv(rect);
            let p1 = Vec2::from(p1);
            let p2 = Vec2::from(p2);
            if let Some(quad) = Quad::from_coords(q.a + p1, q.a + p2).intersection(&q) {
                self.draw.rect(quad, self.cols.text_sel_bg);
            }
        });
        if let Err(e) = result {
            log::error!("text_selected_range: text.highlight_range() -> {e}");
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
        self.draw.text_effects_rgba(rect, text, &effects);
    }

    fn text_cursor(
        &mut self,
        id: &WidgetId,
        rect: Rect,
        text: &TextDisplay,
        _: TextClass,
        byte: usize,
    ) {
        if self.ev.window_has_focus() && !self.w.anim.text_cursor(self.draw.draw, id, byte) {
            return;
        }

        let width = self.w.dims.mark_line;
        let pos = Vec2::conv(rect.pos);
        let p10max = pos.0 + f32::conv(rect.size.0) - width;

        let mut col = self.cols.nav_focus;
        for cursor in text.text_glyph_pos(byte).iter_mut().flatten().rev() {
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

    fn check_box(&mut self, id: &WidgetId, rect: Rect, checked: bool, _: Option<Instant>) {
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

    fn radio_box(
        &mut self,
        id: &WidgetId,
        rect: Rect,
        checked: bool,
        last_change: Option<Instant>,
    ) {
        self.check_box(id, rect, checked, last_change);
    }

    fn mark(&mut self, id: &WidgetId, rect: Rect, style: MarkStyle) {
        let col = if self.ev.is_disabled(id) {
            self.cols.text_disabled
        } else if self.ev.is_hovered(id) {
            self.cols.accent
        } else {
            self.cols.text
        };

        match style {
            MarkStyle::Point(dir) => {
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
        }
    }

    fn scroll_bar(
        &mut self,
        id: &WidgetId,
        id2: &WidgetId,
        rect: Rect,
        h_rect: Rect,
        _: Direction,
    ) {
        let track = Quad::conv(rect);
        self.draw.rect(track, self.cols.frame);

        let handle = Quad::conv(h_rect);
        let state = InputState::new2(self.ev, id, id2);
        let col = self.cols.accent_soft_state(state);
        self.draw.rect(handle, col);
    }

    fn slider(&mut self, id: &WidgetId, id2: &WidgetId, rect: Rect, h_rect: Rect, _: Direction) {
        let track = Quad::conv(rect);
        self.draw.rect(track, self.cols.frame);

        let handle = Quad::conv(h_rect);
        let state = InputState::new2(self.ev, id, id2);
        let col = self.cols.accent_soft_state(state);
        self.draw.rect(handle, col);
    }

    fn progress_bar(&mut self, _: &WidgetId, rect: Rect, dir: Direction, value: f32) {
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
