// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::f32;

use crate::{Dimensions, DimensionsParams, DimensionsWindow, Theme, ThemeColours};
use kas::draw::{
    self, Colour, Draw, DrawRounded, DrawShared, DrawText, DrawTextShared, FontId, Region,
    TextClass, TextProperties,
};
use kas::event::HighlightState;
use kas::geom::{Coord, Rect};
use kas::{Align, Direction, ThemeAction, ThemeApi};

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
            font_size: 18.0,
            cols: ThemeColours::new(),
        }
    }
}

const DIMS: DimensionsParams = DimensionsParams {
    margin: 2.0,
    frame_size: 4.0,
    button_frame: 6.0,
    scrollbar_size: 8.0,
};

pub struct DrawHandle<'a, D: Draw> {
    draw: &'a mut D,
    window: &'a mut DimensionsWindow,
    cols: &'a ThemeColours,
    rect: Rect,
    offset: Coord,
    pass: Region,
}

impl<D: DrawShared + DrawTextShared + 'static> Theme<D> for FlatTheme
where
    D::Draw: DrawRounded + DrawText,
{
    type Window = DimensionsWindow;

    #[cfg(not(feature = "gat"))]
    type DrawHandle = DrawHandle<'static, D::Draw>;
    #[cfg(feature = "gat")]
    type DrawHandle<'a> = DrawHandle<'a, D::Draw>;

    fn init(&mut self, draw: &mut D) {
        self.font_id = crate::load_fonts(draw);
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
            pass: Region::default(),
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
            pass: Region::default(),
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
    /// Draw an edit region with optional navigation highlight.
    /// Return the inner rect.
    fn draw_edit_region(&mut self, outer: Rect, nav_col: Option<Colour>) -> Rect {
        let inner1 = outer.shrink(self.window.dims.frame / 2);
        let inner2 = outer.shrink(self.window.dims.frame);

        self.draw.rect(self.pass, inner1, self.cols.text_area);

        // We draw over the inner rect, taking advantage of the fact that
        // rounded frames get drawn after flat rects.
        self.draw
            .rounded_frame(self.pass, outer, inner2, 0.333, self.cols.frame);

        if let Some(col) = nav_col {
            self.draw.rounded_frame(self.pass, inner1, inner2, 0.0, col);
        }

        inner2
    }
}

impl<'a, D: Draw + DrawRounded + DrawText> draw::DrawHandle for DrawHandle<'a, D> {
    fn draw_device(&mut self) -> (kas::draw::Region, Coord, &mut dyn kas::draw::Draw) {
        (self.pass, self.offset, self.draw)
    }

    fn clip_region(
        &mut self,
        rect: Rect,
        offset: Coord,
        f: &mut dyn FnMut(&mut dyn draw::DrawHandle),
    ) {
        let rect = rect + self.offset;
        let pass = self.draw.add_clip_region(rect);
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
        let outer = rect + self.offset;
        let inner = outer.shrink(self.window.dims.frame);
        self.draw
            .rounded_frame(self.pass, outer, inner, 0.5, self.cols.frame);
    }

    fn text(&mut self, rect: Rect, text: &str, class: TextClass, align: (Align, Align)) {
        let props = TextProperties {
            font: self.window.dims.font_id,
            scale: self.window.dims.font_scale,
            col: match class {
                TextClass::Label => self.cols.label_text,
                TextClass::Button => self.cols.button_text,
                TextClass::Edit | TextClass::EditMulti => self.cols.text,
            },
            align,
            line_wrap: match class {
                TextClass::Label | TextClass::EditMulti => true,
                TextClass::Button | TextClass::Edit => false,
            },
        };
        self.draw.text(rect + self.offset, text, props);
    }

    fn button(&mut self, rect: Rect, highlights: HighlightState) {
        let outer = rect + self.offset;
        let col = self.cols.button_state(highlights);

        let inner = outer.shrink(self.window.dims.button_frame);
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);
        self.draw.rect(self.pass, inner, col);

        if let Some(col) = self.cols.nav_region(highlights) {
            let outer = outer.shrink(self.window.dims.button_frame / 3);
            self.draw.rounded_frame(self.pass, outer, inner, 0.5, col);
        }
    }

    fn edit_box(&mut self, rect: Rect, highlights: HighlightState) {
        self.draw_edit_region(rect + self.offset, self.cols.nav_region(highlights));
    }

    fn checkbox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        let nav_col = self.cols.nav_region(highlights).or_else(|| {
            if checked {
                Some(self.cols.text_area)
            } else {
                None
            }
        });

        let inner = self.draw_edit_region(rect + self.offset, nav_col);

        if let Some(col) = self.cols.check_mark_state(highlights, checked) {
            let radius = (inner.size.0 + inner.size.1) / 16;
            let inner = inner.shrink(self.window.dims.margin + radius);
            let p1 = inner.pos;
            let p2 = inner.pos + inner.size;
            let radius = radius as f32;
            self.draw.rounded_line(self.pass, p1, p2, radius, col);
            self.draw
                .rounded_line(self.pass, Coord(p1.0, p2.1), Coord(p2.0, p1.1), radius, col);
        }
    }

    #[inline]
    fn radiobox(&mut self, rect: Rect, checked: bool, highlights: HighlightState) {
        let nav_col = self.cols.nav_region(highlights).or_else(|| {
            if checked {
                Some(self.cols.text_area)
            } else {
                None
            }
        });

        let inner = self.draw_edit_region(rect + self.offset, nav_col);

        if let Some(col) = self.cols.check_mark_state(highlights, checked) {
            let inner = inner.shrink(self.window.dims.margin);
            self.draw.circle(self.pass, inner, 0.3, col);
        }
    }

    fn scrollbar(
        &mut self,
        _rect: Rect,
        h_rect: Rect,
        _dir: Direction,
        highlights: HighlightState,
    ) {
        // TODO: also draw slider behind handle: needs an extra layer?

        let outer = h_rect + self.offset;
        let half_width = outer.size.0.min(outer.size.1) / 2;
        let inner = outer.shrink(half_width);
        let col = self.cols.scrollbar_state(highlights);
        self.draw.rounded_frame(self.pass, outer, inner, 0.0, col);
        self.draw.rect(self.pass, inner, col);
    }
}
