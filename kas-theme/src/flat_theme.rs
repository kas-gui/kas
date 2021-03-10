// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::f32;
use std::ops::Range;

use crate::{Dimensions, DimensionsParams, DimensionsWindow, Theme, ThemeColours, Window};
use kas::conv::Cast;
use kas::dir::{Direction, Directional};
use kas::draw::{
    self, ClipRegion, Colour, Draw, DrawRounded, DrawShared, DrawText, InputState, Pass,
    SizeHandle, TextClass, ThemeAction, ThemeApi,
};
use kas::geom::*;
use kas::text::format::FormattableText;
use kas::text::{AccelString, Effect, Text, TextApi, TextDisplay};

/// A theme with flat (unshaded) rendering
#[derive(Clone, Debug)]
pub struct FlatTheme {
    pt_size: f32,
    cols: ThemeColours,
}

impl FlatTheme {
    /// Construct
    pub fn new() -> Self {
        FlatTheme {
            pt_size: 12.0,
            cols: ThemeColours::new(),
        }
    }

    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    pub fn with_font_size(mut self, pt_size: f32) -> Self {
        self.pt_size = pt_size;
        self
    }

    /// Set the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    pub fn with_colours(mut self, scheme: &str) -> Self {
        if let Some(scheme) = ThemeColours::open(scheme) {
            self.cols = scheme;
        }
        self
    }
}

const DIMS: DimensionsParams = DimensionsParams {
    outer_margin: 8.0,
    inner_margin: 1.0,
    text_margin: 2.0,
    frame_size: 4.0,
    button_frame: 6.0,
    scrollbar_size: Vec2::splat(8.0),
    slider_size: Vec2(12.0, 25.0),
    progress_bar: Vec2::splat(12.0),
};

pub struct DrawHandle<'a, D: Draw> {
    pub(crate) draw: &'a mut D,
    pub(crate) window: &'a mut DimensionsWindow,
    pub(crate) cols: &'a ThemeColours,
    pub(crate) rect: Rect,
    pub(crate) offset: Offset,
    pub(crate) pass: Pass,
}

impl<D: DrawShared + 'static> Theme<D> for FlatTheme
where
    D::Draw: DrawRounded + DrawText,
{
    type Window = DimensionsWindow;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = DrawHandle<'static, D::Draw>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = DrawHandle<'a, D::Draw>;

    fn init(&mut self, _draw: &mut D) {
        if let Err(e) = kas::text::fonts::fonts().load_default() {
            panic!("Error loading font: {}", e);
        }
    }

    fn new_window(&self, _draw: &mut D::Draw, dpi_factor: f32) -> Self::Window {
        DimensionsWindow::new(DIMS, self.pt_size, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        window.dims = Dimensions::new(DIMS, self.pt_size, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        draw.prepare_fonts();

        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        DrawHandle {
            draw: transmute::<&'a mut D::Draw, &'static mut D::Draw>(draw),
            window: transmute::<&'a mut Self::Window, &'static mut Self::Window>(window),
            cols: transmute::<&'a ThemeColours, &'static ThemeColours>(&self.cols),
            rect,
            offset: Offset::ZERO,
            pass: super::START_PASS,
        }
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        draw.prepare_fonts();

        DrawHandle {
            draw,
            window,
            cols: &self.cols,
            rect,
            offset: Offset::ZERO,
            pass: super::START_PASS,
        }
    }

    fn clear_color(&self) -> Colour {
        self.cols.background
    }
}

impl ThemeApi for FlatTheme {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        self.pt_size = size;
        ThemeAction::ThemeResize
    }

    fn set_colours(&mut self, scheme: &str) -> ThemeAction {
        if let Some(scheme) = ThemeColours::open(scheme) {
            self.cols = scheme;
            ThemeAction::RedrawAll
        } else {
            ThemeAction::None
        }
    }
}

impl<'a, D: Draw + DrawRounded> DrawHandle<'a, D> {
    /// Draw an edit box with optional navigation highlight.
    /// Return the inner rect.
    ///
    /// - `outer`: define position via outer rect
    /// - `bg_col`: colour of background
    /// - `nav_col`: colour of navigation highlight, if visible
    fn draw_edit_box(&mut self, outer: Rect, bg_col: Colour, nav_col: Option<Colour>) -> Quad {
        let outer = Quad::from(outer);
        let inner1 = outer.shrink(self.window.dims.frame as f32 / 2.0);
        let inner2 = outer.shrink(self.window.dims.frame as f32);

        self.draw.rect(self.pass, inner1, bg_col);

        // We draw over the inner rect, taking advantage of the fact that
        // rounded frames get drawn after flat rects.
        self.draw
            .rounded_frame(self.pass, outer, inner2, 0.333, self.cols.frame);

        if let Some(col) = nav_col {
            self.draw.rounded_frame(self.pass, inner1, inner2, 0.0, col);
        }

        inner2
    }

    /// Draw a handle (for slider, scrollbar)
    fn draw_handle(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect + self.offset);
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.scrollbar_state(state);
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(thickness / 4.0);
            self.draw
                .rounded_frame(self.pass, outer, inner, 2.0 / 3.0, col);
        }
    }
}

impl<'a, D: Draw + DrawRounded + DrawText> draw::DrawHandle for DrawHandle<'a, D> {
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        unsafe {
            let mut size_handle = self.window.size_handle();
            f(&mut size_handle);
        }
    }

    fn draw_device(&mut self) -> (kas::draw::Pass, Offset, &mut dyn kas::draw::Draw) {
        (self.pass, self.offset, self.draw)
    }

    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: ClipRegion,
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        let rect = rect + self.offset;
        let depth = self.pass.depth() + super::relative_region_depth(class);
        let pass = self.draw.add_clip_region(rect, depth);
        if depth < self.pass.depth() {
            // draw to depth buffer to enable correct text rendering
            self.draw
                .rect(pass, (rect + self.offset).into(), self.cols.background);
        }
        let mut handle = DrawHandle {
            draw: self.draw,
            window: self.window,
            cols: self.cols,
            rect,
            offset: self.offset - offset,
            pass,
        };
        f(&mut handle);
    }

    fn target_rect(&self) -> Rect {
        // Translate to local coordinates
        self.rect - self.offset
    }

    fn outer_frame(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(self.window.dims.frame as f32);
        self.draw
            .rounded_frame(self.pass, outer, inner, 0.5, self.cols.frame);
    }

    fn menu_frame(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(self.window.dims.frame as f32);
        self.draw
            .rounded_frame(self.pass, outer, inner, 0.5, self.cols.frame);
        let inner = outer.shrink(self.window.dims.frame as f32 / 3.0);
        self.draw.rect(self.pass, inner, self.cols.background);
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        self.draw
            .rounded_frame(self.pass, outer, inner, 0.5, self.cols.frame);
    }

    fn selection_box(&mut self, rect: Rect) {
        let inner = Quad::from(rect + self.offset);
        let outer = inner.grow(self.window.dims.inner_margin.into());
        // TODO: this should use its own colour and a stippled pattern
        let col = self.cols.text_sel_bg;
        self.draw.frame(self.pass, outer, inner, col);
    }

    fn text_offset(
        &mut self,
        pos: Coord,
        bounds: Vec2,
        offset: Offset,
        text: &TextDisplay,
        class: TextClass,
    ) {
        let pos = pos + self.offset;
        let col = self.cols.text_class(class);
        self.draw
            .text(self.pass, pos.into(), bounds, offset.into(), text, col);
    }

    fn text_effects(&mut self, pos: Coord, offset: Offset, text: &dyn TextApi, class: TextClass) {
        self.draw.text_col_effects(
            self.pass,
            (pos + self.offset).into(),
            text.env().bounds.into(),
            offset.into(),
            text.display(),
            self.cols.text_class(class),
            text.effect_tokens(),
        );
    }

    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass) {
        let pos = Vec2::from(pos + self.offset);
        let offset = Vec2::ZERO;
        let bounds = text.env().bounds.into();
        let col = self.cols.text_class(class);
        if state {
            let effects = text.text().effect_tokens();
            self.draw
                .text_col_effects(self.pass, pos, bounds, offset, text.as_ref(), col, effects);
        } else {
            self.draw
                .text(self.pass, pos, bounds, offset, text.as_ref(), col);
        }
    }

    fn text_selected_range(
        &mut self,
        pos: Coord,
        bounds: Vec2,
        offset: Offset,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    ) {
        let pos = Vec2::from(pos + self.offset);
        let offset = Vec2::from(offset);
        let col = self.cols.text_class(class);

        // Draw background:
        for (p1, p2) in &text.highlight_lines(range.clone()) {
            let mut p1 = Vec2::from(*p1) - offset;
            let mut p2 = Vec2::from(*p2) - offset;
            if !p2.gt(Vec2::ZERO) || !p1.lt(bounds) {
                continue;
            }
            p1 = p1.max(Vec2::ZERO);
            p2 = p2.min(bounds);

            let quad = Quad::with_coords(pos + p1, pos + p2);
            self.draw.rect(self.pass, quad, self.cols.text_sel_bg);
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
        self.draw
            .text_effects(self.pass, pos, bounds, offset, text, &effects);
    }

    fn edit_marker(
        &mut self,
        pos: Coord,
        mut bounds: Vec2,
        offset: Offset,
        text: &TextDisplay,
        class: TextClass,
        byte: usize,
    ) {
        let width = self.window.dims.font_marker_width;
        let p = Vec2::from(pos + self.offset);
        bounds.0 += width;
        let bounds = Quad::with_pos_and_size(p, bounds);
        let pos = Vec2::from(pos - offset + self.offset);

        let mut col = self.cols.text_class(class);
        for cursor in text.text_glyph_pos(byte).rev() {
            let mut p1 = pos + Vec2::from(cursor.pos);
            let mut p2 = p1;
            p1.1 -= cursor.ascent;
            p2.1 -= cursor.descent;
            p2.0 += width;
            let quad = Quad::with_coords(p1, p2);
            if let Some(quad) = bounds.intersection(&quad) {
                self.draw.rect(self.pass, quad, col);
            }
            if cursor.embedding_level() > 0 {
                // Add a hat to indicate directionality.
                let height = width;
                let quad = if cursor.is_ltr() {
                    Quad::with_coords(Vec2(p2.0, p1.1), Vec2(p2.0 + width, p1.1 + height))
                } else {
                    Quad::with_coords(Vec2(p1.0 - width, p1.1), Vec2(p1.0, p1.1 + height))
                };
                if let Some(quad) = bounds.intersection(&quad) {
                    self.draw.rect(self.pass, quad, col);
                }
            }
            // hack to make secondary marker grey:
            col = self.cols.button_disabled;
        }
    }

    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        if let Some(col) = self.cols.menu_entry(state) {
            let quad = Quad::from(rect + self.offset);
            self.draw.rect(self.pass, quad, col);
        }
    }

    fn button(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect + self.offset);
        let col = self.cols.button_state(state);

        let inner = outer.shrink(self.window.dims.button_frame as f32);
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);
        self.draw.rect(self.pass, inner, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(self.window.dims.button_frame as f32 / 3.0);
            self.draw.rounded_frame(self.pass, outer, inner, 0.5, col);
        }
    }

    fn edit_box(&mut self, rect: Rect, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        self.draw_edit_box(rect + self.offset, bg_col, self.cols.nav_region(state));
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect + self.offset, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let radius = inner.size().sum() * (1.0 / 16.0);
            let inner = inner.shrink(self.window.dims.inner_margin as f32 + radius);
            self.draw
                .rounded_line(self.pass, inner.a, inner.b, radius, col);
            self.draw
                .rounded_line(self.pass, inner.ab(), inner.ba(), radius, col);
        }
    }

    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect + self.offset, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            let inner = inner.shrink(self.window.dims.inner_margin as f32);
            self.draw.circle(self.pass, inner, 0.3, col);
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, _dir: Direction, state: InputState) {
        // track
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.frame;
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn slider(&mut self, rect: Rect, h_rect: Rect, dir: Direction, state: InputState) {
        // track
        let mut outer = Quad::from(rect + self.offset);
        outer = match dir.is_horizontal() {
            true => outer.shrink_vec(Vec2(0.0, outer.size().1 * (3.0 / 8.0))),
            false => outer.shrink_vec(Vec2(outer.size().0 * (3.0 / 8.0), 0.0)),
        };
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let col = self.cols.frame;
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn progress_bar(&mut self, rect: Rect, dir: Direction, _: InputState, value: f32) {
        let outer = Quad::from(rect + self.offset);
        let mut inner = outer.shrink(self.window.dims.frame as f32);
        let col = self.cols.frame;
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);

        if dir.is_horizontal() {
            inner.b.0 = inner.a.0 + value * (inner.b.0 - inner.a.0);
        } else {
            inner.b.1 = inner.a.1 + value * (inner.b.1 - inner.a.1);
        }
        let col = self.cols.button;
        self.draw.rect(self.pass, inner, col);
    }
}
