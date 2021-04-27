// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shaded theme

use std::f32;
use std::ops::Range;

use crate::{Dimensions, DimensionsParams, DimensionsWindow, Theme, ThemeColours, Window};
use kas::dir::{Direction, Directional};
use kas::draw::{
    self, Colour, Draw, DrawRounded, DrawShaded, DrawShared, ImageId, InputState, Pass, SizeHandle,
    TextClass, ThemeAction, ThemeApi,
};
use kas::geom::*;
use kas::text::{AccelString, Text, TextApi, TextDisplay};

/// A theme using simple shading to give apparent depth to elements
#[derive(Clone, Debug)]
pub struct ShadedTheme {
    pt_size: f32,
    cols: ThemeColours,
}

impl ShadedTheme {
    /// Construct
    pub fn new() -> Self {
        ShadedTheme {
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
    outer_margin: 6.0,
    inner_margin: 1.2,
    text_margin: 2.0,
    frame_size: 5.0,
    button_frame: 5.0,
    scrollbar_size: Vec2::splat(8.0),
    slider_size: Vec2(12.0, 25.0),
    progress_bar: Vec2::splat(12.0),
};

pub struct DrawHandle<'a, D: DrawShared> {
    shared: &'a mut D,
    draw: &'a mut D::Draw,
    window: &'a mut DimensionsWindow,
    cols: &'a ThemeColours,
    rect: Rect,
    offset: Offset,
    pass: Pass,
}

impl<D: DrawShared> Theme<D> for ShadedTheme
where
    D::Draw: DrawRounded + DrawShaded,
{
    type Window = DimensionsWindow;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = DrawHandle<'static, D>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = DrawHandle<'a, D>;

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
        shared: &'a mut D,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle {
        // We extend lifetimes (unsafe) due to the lack of associated type generics.
        unsafe fn extend_lifetime<'b, T>(r: &'b mut T) -> &'static mut T {
            std::mem::transmute::<&'b mut T, &'static mut T>(r)
        }

        DrawHandle {
            shared: extend_lifetime(shared),
            draw: extend_lifetime(draw),
            window: extend_lifetime(window),
            cols: std::mem::transmute::<&'a ThemeColours, &'static ThemeColours>(&self.cols),
            rect,
            offset: Offset::ZERO,
            pass: Pass::new(0),
        }
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut D,
        draw: &'a mut D::Draw,
        window: &'a mut Self::Window,
        rect: Rect,
    ) -> Self::DrawHandle<'a> {
        DrawHandle {
            shared,
            draw,
            window,
            cols: &self.cols,
            rect,
            offset: Offset::ZERO,
            pass: Pass::new(0),
        }
    }

    fn clear_color(&self) -> Colour {
        self.cols.background
    }
}

impl ThemeApi for ShadedTheme {
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

impl<'a, D: DrawShared> DrawHandle<'a, D>
where
    D::Draw: DrawRounded + DrawShaded,
{
    // Type-cast to flat_theme's DrawHandle. Should be equivalent to transmute.
    fn as_flat<'b, 'c>(&'b mut self) -> super::flat_theme::DrawHandle<'c, D>
    where
        'a: 'c,
        'b: 'c,
    {
        super::flat_theme::DrawHandle {
            shared: *&mut self.shared,
            draw: *&mut self.draw,
            window: *&mut self.window,
            cols: *&self.cols,
            rect: self.rect,
            offset: self.offset,
            pass: self.pass,
        }
    }

    /// Draw an edit box with optional navigation highlight.
    /// Return the inner rect.
    ///
    /// - `outer`: define position via outer rect
    /// - `bg_col`: colour of background
    /// - `nav_col`: colour of navigation highlight, if visible
    fn draw_edit_box(&mut self, outer: Rect, bg_col: Colour, nav_col: Option<Colour>) -> Quad {
        let mut outer = Quad::from(outer);
        let mut inner = outer.shrink(self.window.dims.frame as f32);

        self.draw
            .shaded_square_frame(self.pass, outer, inner, (-0.6, 0.0), self.cols.background);

        if let Some(col) = nav_col {
            outer = inner;
            inner = outer.shrink(self.window.dims.inner_margin as f32);
            self.draw.frame(self.pass, outer, inner, col);
        }

        self.draw.rect(self.pass, inner, bg_col);
        inner
    }

    /// Draw a handle (for slider, scrollbar)
    fn draw_handle(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect + self.offset);
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.scrollbar_state(state);
        self.draw
            .shaded_round_frame(self.pass, outer, inner, (0.0, 0.6), col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(thickness / 4.0);
            self.draw
                .rounded_frame(self.pass, outer, inner, 2.0 / 3.0, col);
        }
    }
}

impl<'a, D: DrawShared> draw::DrawHandle for DrawHandle<'a, D>
where
    D::Draw: DrawRounded + DrawShaded,
{
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        unsafe {
            let mut size_handle = self.window.size_handle(self.shared);
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
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        let rect = rect + self.offset;
        let pass = self.draw.add_clip_region(rect);
        let mut handle = DrawHandle {
            shared: self.shared,
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
        let norm = (0.7, -0.7);
        let col = self.cols.background;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);
    }

    fn menu_frame(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(self.window.dims.frame as f32);
        let norm = (0.7, 0.0);
        let col = self.cols.background;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);
        self.draw.rect(self.pass, inner, self.cols.background);
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.background;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);
    }

    fn nav_frame(&mut self, rect: Rect, state: InputState) {
        self.as_flat().nav_frame(rect, state);
    }

    fn selection_box(&mut self, rect: Rect) {
        self.as_flat().selection_box(rect);
    }

    fn text(&mut self, pos: Coord, text: &TextDisplay, class: TextClass) {
        self.as_flat().text(pos, text, class);
    }

    fn text_effects(&mut self, pos: Coord, text: &dyn TextApi, class: TextClass) {
        self.as_flat().text_effects(pos, text, class);
    }

    fn text_accel(&mut self, pos: Coord, text: &Text<AccelString>, state: bool, class: TextClass) {
        self.as_flat().text_accel(pos, text, state, class);
    }

    fn text_selected_range(
        &mut self,
        pos: Coord,
        text: &TextDisplay,
        range: Range<usize>,
        class: TextClass,
    ) {
        self.as_flat().text_selected_range(pos, text, range, class);
    }

    fn edit_marker(&mut self, pos: Coord, text: &TextDisplay, class: TextClass, byte: usize) {
        self.as_flat().edit_marker(pos, text, class, byte);
    }

    fn menu_entry(&mut self, rect: Rect, state: InputState) {
        self.as_flat().menu_entry(rect, state);
    }

    fn button(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(self.window.dims.button_frame as f32);
        let col = self.cols.button_state(state);

        self.draw
            .shaded_round_frame(self.pass, outer, inner, (0.0, 0.6), col);
        self.draw.rect(self.pass, inner, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(self.window.dims.inner_margin as f32);
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
            self.draw.shaded_square(self.pass, inner, (0.0, 0.4), col);
        }
    }

    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect + self.offset, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            self.draw.shaded_circle(self.pass, inner, (0.0, 1.0), col);
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, _dir: Direction, state: InputState) {
        // track
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.background;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);

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
        let norm = (0.0, -0.7);
        let col = self.cols.background;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn progress_bar(&mut self, rect: Rect, dir: Direction, _: InputState, value: f32) {
        let mut outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.frame;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, norm, col);

        if dir.is_horizontal() {
            outer.b.0 = outer.a.0 + value * (outer.b.0 - outer.a.0);
        } else {
            outer.b.1 = outer.a.1 + value * (outer.b.1 - outer.a.1);
        }
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.button;
        self.draw
            .shaded_round_frame(self.pass, outer, inner, (0.0, 0.6), col);
    }

    fn image(&mut self, id: ImageId, rect: Rect) {
        self.as_flat().image(id, rect);
    }
}
