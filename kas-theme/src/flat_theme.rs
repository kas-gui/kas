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
use kas::draw::{
    self, ClipRegion, Colour, Draw, DrawRounded, DrawShared, DrawText, InputState, Pass,
    SizeHandle, TextClass,
};
use kas::geom::*;
use kas::text::{FontId, PreparedText};
use kas::{Direction, Directional, ThemeAction, ThemeApi};

/// A theme with flat (unshaded) rendering
#[derive(Clone, Debug)]
pub struct FlatTheme {
    font_id: FontId,
    font_size: f32,
    cols: ThemeColours,
}

impl FlatTheme {
    /// Construct
    pub fn new() -> Self {
        FlatTheme {
            font_id: Default::default(),
            font_size: 12.0,
            cols: ThemeColours::new(),
        }
    }
}

const DIMS: DimensionsParams = DimensionsParams {
    outer_margin: 8.0,
    inner_margin: 1.0,
    frame_size: 4.0,
    button_frame: 6.0,
    scrollbar_size: Vec2::splat(8.0),
    slider_size: Vec2(12.0, 25.0),
};

pub struct DrawHandle<'a, D: Draw> {
    pub(crate) draw: &'a mut D,
    pub(crate) window: &'a mut DimensionsWindow,
    pub(crate) cols: &'a ThemeColours,
    pub(crate) rect: Rect,
    pub(crate) offset: Coord,
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
        self.font_id = kas::text::fonts().load_default().unwrap();
    }

    fn new_window(&self, _draw: &mut D::Draw, dpi_factor: f32) -> Self::Window {
        DimensionsWindow::new(DIMS, self.font_id, self.font_size, dpi_factor)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        window.dims = Dimensions::new(DIMS, self.font_id, self.font_size, dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle<'a>(
        &'a self,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        use std::mem::transmute;
        DrawHandle {
            draw: transmute::<&'a mut D::Draw, &'static mut D::Draw>(draw),
            window: transmute::<&'a mut Self::Window, &'static mut Self::Window>(window),
            cols: transmute::<&'a ThemeColours, &'static ThemeColours>(&self.cols),
            rect,
            offset: Coord::ZERO,
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
        DrawHandle {
            draw,
            window,
            cols: &self.cols,
            rect,
            offset: Coord::ZERO,
            pass: super::START_PASS,
        }
    }

    fn clear_colour(&self) -> Colour {
        self.cols.background
    }
}

impl ThemeApi for FlatTheme {
    fn set_font_size(&mut self, size: f32) -> ThemeAction {
        self.font_size = size;
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

    fn draw_device(&mut self) -> (kas::draw::Pass, Coord, &mut dyn kas::draw::Draw) {
        (self.pass, self.offset, self.draw)
    }

    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
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

    fn text(&mut self, pos: Coord, text: &PreparedText, class: TextClass) {
        let pos = pos + self.offset;
        let col = self.cols.text_class(class);
        self.draw.text(self.pass, pos.into(), col, text);
    }

    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &PreparedText,
        range: Range<usize>,
        class: TextClass,
    ) {
        let pos = pos + self.offset;
        let col = self.cols.text_class(class);

        // Draw background:
        for quad in &text.highlight_lines(pos, range) {
            self.draw.rect(self.pass, *quad, self.cols.text_sel_bg);
        }

        // TODO: which should use self.cols.text_sel for the selected range!
        self.draw.text(self.pass, pos.into(), col, text);
    }

    fn edit_marker(&mut self, pos: Coord, text: &PreparedText, class: TextClass, byte: usize) {
        let col = self.cols.text_class(class);
        for (mut p1, ascent, descent) in text.text_glyph_pos(pos + self.offset, byte) {
            let mut p2 = p1;
            p1.1 -= ascent;
            p2.1 -= descent;
            p2.0 += self.window.dims.font_marker_width;
            let quad = Quad::with_coords(p1, p2);
            self.draw.rect(self.pass, quad, col);
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
            let radius = radius as f32;
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
}
