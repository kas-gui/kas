// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use std::f32;
use unicode_segmentation::GraphemeCursor;
use wgpu_glyph::ab_glyph::{Glyph, PxScale, PxScaleFont, ScaleFont};
use wgpu_glyph::{
    Extra, GlyphCruncher, HorizontalAlign, Layout, Section, SectionGlyph, Text, VerticalAlign,
};

use super::{CustomPipe, CustomWindow, DrawPipe, DrawWindow};
use kas::draw::{DrawText, DrawTextShared, FontArc, FontId, Pass, TextProperties};
use kas::geom::{Coord, Rect, Vec2};
use kas::Align;

impl<C: CustomPipe + 'static> DrawTextShared for DrawPipe<C> {
    fn load_font(&mut self, font: FontArc) -> FontId {
        let id = FontId(self.fonts.len());
        self.fonts.push(font);
        id
    }
}

fn make_section(pass: Pass, rect: Rect, text: &str, props: TextProperties) -> Section {
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

    let text = vec![Text {
        text,
        scale: PxScale::from(props.scale),
        font_id: wgpu_glyph::FontId(props.font.0),
        extra: Extra {
            color: props.col.into(),
            z: pass.depth(),
        },
    }];

    Section {
        screen_position: Vec2::from(text_pos).into(),
        bounds: Vec2::from(bounds).into(),
        layout,
        text,
    }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text(&mut self, pass: Pass, rect: Rect, text: &str, props: TextProperties) {
        self.glyph_brush
            .queue(make_section(pass, rect, text, props));
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

        let text = vec![Text {
            text,
            scale: PxScale::from(font_scale),
            font_id: wgpu_glyph::FontId(font_id.0),
            extra: Default::default(),
        }];

        self.glyph_brush
            .glyph_bounds(Section {
                screen_position: (0.0, 0.0),
                bounds,
                layout,
                text,
            })
            .map(|rect| (Vec2(rect.min.x, rect.min.y), Vec2(rect.max.x, rect.max.y)))
            .map(|(min, max)| max - min)
            .unwrap_or(Vec2::splat(0.0))
            .into()
    }

    fn text_glyph_pos(
        &mut self,
        rect: Rect,
        text: &str,
        props: TextProperties,
        byte: usize,
    ) -> Vec2 {
        if byte == 0 {
            // Short-cut. We also cannot iterate since there may be no glyphs.
            return rect.pos.into();
        }
        let pass = Pass::new_pass_with_depth(0, 0.0); // values are unimportant
        let mut iter = self
            .glyph_brush
            .glyphs(make_section(pass, rect, text, props));

        let mut advance = false;
        let mut glyph;
        if byte < text.len() {
            // Tiny HACK: line-breaks don't have glyphs
            let b = text.as_bytes()[byte];
            if b == b'\r' || b == b'\n' {
                advance = true;
            }

            glyph = iter.next().unwrap().clone();
            for next in iter {
                assert_eq!(next.section_index, 0);
                if byte < next.byte_index {
                    // Use the previous glyph, e.g. if in the middle of a
                    // multi-byte sequence or index is a combining diacritic.
                    break;
                }
                glyph = next.clone();
            }
        } else {
            advance = true;
            glyph = iter.last().unwrap().clone();
        }

        let mut pos = glyph.glyph.position;
        let font = self.glyph_brush.fonts()[glyph.font_id.0].clone();
        let scale = props.scale;
        let scale_font = PxScaleFont { font, scale };
        if advance {
            pos.x += scale_font.h_advance(glyph.glyph.id);
        }
        pos.y -= scale_font.ascent();
        return Vec2(pos.x, pos.y);
    }

    fn text_index_nearest(
        &mut self,
        rect: Rect,
        text: &str,
        props: TextProperties,
        pos: Vec2,
    ) -> usize {
        if text.len() == 0 {
            return 0; // short-cut
        }
        let scale_font = PxScaleFont {
            font: self.glyph_brush.fonts()[props.font.0].clone(),
            scale: props.scale,
        };
        let base_to_mid = -0.5 * scale_font.ascent();

        let pass = Pass::new_pass_with_depth(0, 0.0); // values are unimportant
        let mut iter = self
            .glyph_brush
            .glyphs(make_section(pass, rect, text, props));

        // Find the (horiz, vert) distance between pos and the glyph.
        let dist = |glyph: &Glyph| {
            let p = glyph.position;
            let glyph_pos = Vec2(p.x, p.y + base_to_mid);
            (pos - glyph_pos).abs()
        };
        let test_best = |best: Vec2, glyph: &Glyph| {
            let dist = dist(glyph);
            if dist.1 < best.1 {
                Some(dist)
            } else if dist.1 == best.1 && dist.0 < best.0 {
                Some(dist)
            } else {
                None
            }
        };

        let mut last: SectionGlyph = iter.next().unwrap().clone();
        let mut last_y = last.glyph.position.y;
        let mut best = (last.byte_index, dist(&last.glyph));
        for next in iter {
            // Heuristic to detect a new line. This is a HACK to handle
            // multi-line texts since line-end positions are not represented by
            // virtual glyphs (unlike spaces).
            if (next.glyph.position.y - last_y).abs() > base_to_mid {
                last.glyph.position.x += scale_font.h_advance(last.glyph.id);
                if let Some(new_best) = test_best(best.1, &last.glyph) {
                    let mut cursor = GraphemeCursor::new(last.byte_index, text.len(), true);
                    let byte = cursor
                        .next_boundary(&text, 0)
                        .unwrap()
                        .unwrap_or(last.byte_index);
                    best = (byte, new_best);
                }
            }

            last = next.clone();
            last_y = last.glyph.position.y;
            if let Some(new_best) = test_best(best.1, &last.glyph) {
                best = (last.byte_index, new_best);
            }
        }

        // We must also consider the position after the last glyph
        last.glyph.position.x += scale_font.h_advance(last.glyph.id);
        if let Some(new_best) = test_best(best.1, &last.glyph) {
            best = (text.len(), new_best);
        }

        assert!(
            best.0 <= text.len(),
            "text_index_nearest: index beyond text length!"
        );
        best.0
    }
}
