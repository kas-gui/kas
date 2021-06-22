// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Shaded theme

use std::f32;
use std::ops::Range;

use crate::{ColorsLinear, Config, DimensionsParams, DimensionsWindow, FlatTheme, Theme, Window};
use kas::dir::{Direction, Directional};
use kas::draw::{self, color::Rgba, *};
use kas::geom::*;
use kas::text::{AccelString, Text, TextApi, TextDisplay};
use kas::TkAction;

/// A theme using simple shading to give apparent depth to elements
#[derive(Clone, Debug)]
pub struct ShadedTheme {
    flat: FlatTheme,
}

impl ShadedTheme {
    /// Construct
    pub fn new() -> Self {
        let flat = FlatTheme::new();
        ShadedTheme { flat }
    }

    /// Set font size
    ///
    /// Units: Points per Em (standard unit of font size)
    pub fn with_font_size(mut self, pt_size: f32) -> Self {
        self.flat.config.set_font_size(pt_size);
        self
    }

    /// Set the colour scheme
    ///
    /// If no scheme by this name is found the scheme is left unchanged.
    pub fn with_colours(mut self, scheme: &str) -> Self {
        self.flat.config.set_active_scheme(scheme);
        if let Some(scheme) = self.flat.config.get_color_scheme(scheme) {
            self.flat.cols = scheme.into();
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

pub struct DrawHandle<'a, DS: DrawableShared> {
    shared: &'a mut DrawShared<DS>,
    draw: Draw<'a, DS::Draw>,
    window: &'a mut DimensionsWindow,
    cols: &'a ColorsLinear,
    offset: Offset,
}

impl<DS: DrawableShared> Theme<DS> for ShadedTheme
where
    DS::Draw: DrawableRounded + DrawableShaded,
{
    type Config = Config;
    type Window = DimensionsWindow;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = DrawHandle<'static, DS>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = DrawHandle<'a, DS>;

    fn config(&self) -> std::borrow::Cow<Self::Config> {
        <FlatTheme as Theme<DS>>::config(&self.flat)
    }

    fn apply_config(&mut self, config: &Self::Config) -> TkAction {
        <FlatTheme as Theme<DS>>::apply_config(&mut self.flat, config)
    }

    fn init(&mut self, shared: &mut DrawShared<DS>) {
        <FlatTheme as Theme<DS>>::init(&mut self.flat, shared)
    }

    fn new_window(&self, dpi_factor: f32) -> Self::Window {
        let fonts = self.flat.fonts.as_ref().clone().unwrap().clone();
        DimensionsWindow::new(DIMS, self.flat.config.font_size(), dpi_factor, fonts)
    }

    fn update_window(&self, window: &mut Self::Window, dpi_factor: f32) {
        window.update(DIMS, self.flat.config.font_size(), dpi_factor);
    }

    #[cfg(not(feature = "gat"))]
    unsafe fn draw_handle(
        &self,
        shared: &'static mut DrawShared<DS>,
        draw: Draw<'static, DS::Draw>,
        window: &'static mut Self::Window,
    ) -> Self::DrawHandle {
        DrawHandle {
            shared,
            draw,
            window,
            cols: std::mem::transmute::<&ColorsLinear, &'static ColorsLinear>(&self.flat.cols),
            offset: Offset::ZERO,
        }
    }
    #[cfg(feature = "gat")]
    fn draw_handle<'a>(
        &'a self,
        shared: &'a mut DrawShared<DS>,
        draw: Draw<'a, DS::Draw>,
        window: &'a mut Self::Window,
    ) -> Self::DrawHandle<'a> {
        DrawHandle {
            shared,
            draw,
            window,
            cols: &self.flat.cols,
            offset: Offset::ZERO,
        }
    }

    fn clear_color(&self) -> Rgba {
        <FlatTheme as Theme<DS>>::clear_color(&self.flat)
    }
}

impl ThemeApi for ShadedTheme {
    fn set_font_size(&mut self, pt_size: f32) -> TkAction {
        self.flat.set_font_size(pt_size)
    }

    fn list_schemes(&self) -> Vec<&str> {
        self.flat.list_schemes()
    }

    fn set_scheme(&mut self, name: &str) -> TkAction {
        self.flat.set_scheme(name)
    }
}

impl<'a, DS: DrawableShared> DrawHandle<'a, DS>
where
    DS::Draw: DrawableRounded + DrawableShaded,
{
    // Type-cast to flat_theme's DrawHandle. Should be equivalent to transmute.
    fn as_flat<'b, 'c>(&'b mut self) -> super::flat_theme::DrawHandle<'c, DS>
    where
        'a: 'c,
        'b: 'c,
    {
        super::flat_theme::DrawHandle {
            shared: *&mut self.shared,
            draw: self.draw.reborrow(),
            window: *&mut self.window,
            cols: *&self.cols,
            offset: self.offset,
        }
    }

    /// Draw an edit box with optional navigation highlight.
    /// Return the inner rect.
    ///
    /// - `outer`: define position via outer rect
    /// - `bg_col`: colour of background
    /// - `nav_col`: colour of navigation highlight, if visible
    fn draw_edit_box(&mut self, outer: Rect, bg_col: Rgba, nav_col: Option<Rgba>) -> Quad {
        let mut outer = Quad::from(outer);
        let mut inner = outer.shrink(self.window.dims.frame as f32);

        let col = self.cols.background;
        self.draw
            .shaded_square_frame(outer, inner, (-0.6, 0.0), col, col);

        if let Some(col) = nav_col {
            outer = inner;
            inner = outer.shrink(self.window.dims.inner_margin as f32);
            self.draw.frame(outer, inner, col);
        }

        self.draw.rect(inner, bg_col);
        inner
    }

    /// Draw a handle (for slider, scrollbar)
    fn draw_handle(&mut self, rect: Rect, state: InputState) {
        let outer = Quad::from(rect + self.offset);
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.scrollbar_state(state);
        self.draw.shaded_round_frame(outer, inner, (0.0, 0.6), col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(thickness / 4.0);
            self.draw.rounded_frame(outer, inner, 0.6, col);
        }
    }
}

impl<'a, DS: DrawableShared> draw::DrawHandle for DrawHandle<'a, DS>
where
    DS::Draw: DrawableRounded + DrawableShaded,
{
    fn size_handle_dyn(&mut self, f: &mut dyn FnMut(&mut dyn SizeHandle)) {
        unsafe {
            let mut size_handle = self.window.size_handle(self.shared);
            f(&mut size_handle);
        }
    }

    fn draw_device<'b>(&'b mut self) -> (Offset, Draw<'b, dyn Drawable>, &mut dyn DrawSharedT) {
        (self.offset, self.draw.upcast_base(), self.shared)
    }

    fn with_clip_region(
        &mut self,
        rect: Rect,
        offset: Offset,
        class: RegionClass,
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        let inner_rect = rect + self.offset;
        let outer_rect = match class {
            RegionClass::ScrollRegion => inner_rect,
            RegionClass::Overlay => inner_rect.expand(self.window.dims.frame),
        };
        let mut draw = self.draw.new_clip_region(outer_rect, class);

        if class == RegionClass::Overlay {
            let outer = Quad::from(outer_rect);
            let inner = Quad::from(inner_rect);
            let norm = (0.0, 0.0);
            draw.shaded_square_frame(outer, inner, norm, Rgba::TRANSPARENT, Rgba::BLACK);
            draw.rect(inner, self.cols.background);
        }

        let mut handle = DrawHandle {
            shared: self.shared,
            draw,
            window: self.window,
            cols: self.cols,
            offset: self.offset - offset,
        };
        f(&mut handle);
    }

    fn target_rect(&self) -> Rect {
        // Translate to local coordinates
        self.draw.clip_rect() - self.offset
    }

    fn outer_frame(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(self.window.dims.frame as f32);
        let norm = (0.7, -0.7);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, norm, col);
    }

    fn separator(&mut self, rect: Rect) {
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, norm, col);
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

        self.draw.shaded_round_frame(outer, inner, (0.0, 0.6), col);
        self.draw.rect(inner, col);

        if let Some(col) = self.cols.nav_region(state) {
            let outer = outer.shrink(self.window.dims.inner_margin as f32);
            self.draw.rounded_frame(outer, inner, 0.6, col);
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
            self.draw.shaded_square(inner, (0.0, 0.4), col);
        }
    }

    fn radiobox(&mut self, rect: Rect, checked: bool, state: InputState) {
        let bg_col = self.cols.bg_col(state);
        let nav_col = self.cols.nav_region(state).or(Some(bg_col));

        let inner = self.draw_edit_box(rect + self.offset, bg_col, nav_col);

        if let Some(col) = self.cols.check_mark_state(state, checked) {
            self.draw.shaded_circle(inner, (0.0, 1.0), col);
        }
    }

    fn scrollbar(&mut self, rect: Rect, h_rect: Rect, _dir: Direction, state: InputState) {
        // track
        let outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.background;
        self.draw.shaded_round_frame(outer, inner, norm, col);

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
        self.draw.shaded_round_frame(outer, inner, norm, col);

        // handle
        self.draw_handle(h_rect, state);
    }

    fn progress_bar(&mut self, rect: Rect, dir: Direction, _: InputState, value: f32) {
        let mut outer = Quad::from(rect + self.offset);
        let inner = outer.shrink(outer.size().min_comp() / 2.0);
        let norm = (0.0, -0.7);
        let col = self.cols.frame;
        self.draw.shaded_round_frame(outer, inner, norm, col);

        if dir.is_horizontal() {
            outer.b.0 = outer.a.0 + value * (outer.b.0 - outer.a.0);
        } else {
            outer.b.1 = outer.a.1 + value * (outer.b.1 - outer.a.1);
        }
        let thickness = outer.size().min_comp() / 2.0;
        let inner = outer.shrink(thickness);
        let col = self.cols.button;
        self.draw.shaded_round_frame(outer, inner, (0.0, 0.6), col);
    }

    fn image(&mut self, id: ImageId, rect: Rect) {
        self.as_flat().image(id, rect);
    }
}
