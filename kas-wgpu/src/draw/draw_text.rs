// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use std::f32;
use wgpu_glyph::{GlyphCruncher, HorizontalAlign, Layout, Scale, Section, VerticalAlign};

use super::{CustomPipe, CustomWindow, DrawPipe, DrawWindow, Vec2};
use kas::draw::{DrawText, DrawTextShared, Font, FontId, TextProperties};
use kas::geom::{Coord, Rect};
use kas::Align;

impl<C: CustomPipe + 'static> DrawTextShared for DrawPipe<C> {
    fn load_font(&mut self, font: Font<'static>) -> FontId {
        let id = FontId(self.fonts.len());
        self.fonts.push(font);
        id
    }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text(&mut self, rect: Rect, text: &str, props: TextProperties) {
        let bounds = Coord::from(rect.size);

        // TODO: support justified alignment
        let (h_align, h_offset) = match props.align.0 {
            Align::Begin | Align::Stretch => (HorizontalAlign::Left, 0),
            Align::Centre => (HorizontalAlign::Center, bounds.0 / 2),
            Align::End => (HorizontalAlign::Right, bounds.0),
        };
        let (v_align, v_offset) = match props.align.1 {
            Align::Begin | Align::Stretch => (VerticalAlign::Top, 0),
            Align::Centre => (VerticalAlign::Center, bounds.1 / 2),
            Align::End => (VerticalAlign::Bottom, bounds.1),
        };

        let text_pos = rect.pos + Coord(h_offset, v_offset);

        let layout = match props.line_wrap {
            true => Layout::default_wrap(),
            false => Layout::default_single_line(),
        };
        let layout = layout.h_align(h_align).v_align(v_align);

        self.glyph_brush.queue(Section {
            text,
            screen_position: Vec2::from(text_pos).into(),
            bounds: Vec2::from(bounds).into(),
            scale: Scale::uniform(props.scale),
            color: props.col.into(),
            z: 0.0,
            layout,
            font_id: wgpu_glyph::FontId(props.font.0),
        });
    }

    #[inline]
    fn text_bound(
        &mut self,
        text: &str,
        font_id: FontId,
        font_scale: f32,
        bounds: (f32, f32),
        line_wrap: bool,
    ) -> (f32, f32) {
        let layout = match line_wrap {
            true => Layout::default_wrap(),
            false => Layout::default_single_line(),
        };

        self.glyph_brush
            .glyph_bounds(Section {
                text,
                screen_position: (0.0, 0.0),
                bounds,
                scale: Scale::uniform(font_scale),
                color: Default::default(),
                z: 0.0,
                layout,
                font_id: wgpu_glyph::FontId(font_id.0),
            })
            .map(|rect| (Vec2(rect.min.x, rect.min.y), Vec2(rect.max.x, rect.max.y)))
            .map(|(min, max)| max - min)
            .unwrap_or(Vec2::splat(0.0))
            .into()
    }
}
