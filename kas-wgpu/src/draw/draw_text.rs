// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text drawing API for `kas_wgpu`

use unicode_segmentation::GraphemeCursor;
use wgpu_glyph::ab_glyph::{self, Glyph, PxScale, PxScaleFont, ScaleFont};
use wgpu_glyph::{
    Extra, FontId, GlyphCruncher, HorizontalAlign, Layout, Section, SectionGlyph, Text,
    VerticalAlign,
};

use super::{CustomWindow, DrawWindow};
use kas::draw::{Colour, DrawText, Pass, TextSection};
use kas::geom::{Coord, Vec2};
use kas::text::PreparedText;
use kas::Align;

// Local trait so that we can implement for foreign types
trait MyInto<T> {
    fn my_into(self) -> T;
}

impl MyInto<PxScale> for kas::draw::PxScale {
    fn my_into(self) -> PxScale {
        let kas::draw::PxScale { x, y } = self;
        PxScale { x, y }
    }
}

impl MyInto<FontId> for kas::text::FontId {
    fn my_into(self) -> FontId {
        FontId(self.get())
    }
}

impl MyInto<ab_glyph::Point> for Vec2 {
    fn my_into(self) -> ab_glyph::Point {
        ab_glyph::Point {
            x: self.0,
            y: self.1,
        }
    }
}

fn make_section<'a>(pass: Pass, ts: &'a TextSection) -> Section<'a> {
    let bounds = Coord::from(ts.rect.size);

    // TODO: support justified alignment
    let (h_align, h_offset) = match ts.align.0 {
        Align::Default | Align::TL | Align::Stretch => (HorizontalAlign::Left, 0),
        Align::Centre => (HorizontalAlign::Center, bounds.0 / 2),
        Align::BR => (HorizontalAlign::Right, bounds.0),
    };
    let (v_align, v_offset) = match ts.align.1 {
        Align::Default | Align::TL | Align::Stretch => (VerticalAlign::Top, 0),
        Align::Centre => (VerticalAlign::Center, bounds.1 / 2),
        Align::BR => (VerticalAlign::Bottom, bounds.1),
    };

    let text_pos = ts.rect.pos + Coord(h_offset, v_offset);

    let layout = match ts.line_wrap {
        true => Layout::default_wrap(),
        false => Layout::default_single_line(),
    };
    let layout = layout.h_align(h_align).v_align(v_align);

    let text = ts.text;
    let text = ts
        .parts
        .iter()
        .map(|part| Text {
            text: &text[part.range()],
            scale: part.scale.my_into(),
            font_id: part.font.my_into(),
            extra: Extra {
                color: part.col.into(),
                z: pass.depth(),
            },
        })
        .collect();

    Section {
        screen_position: Vec2::from(text_pos).into(),
        bounds: Vec2::from(bounds).into(),
        layout,
        text,
    }
}

impl<CW: CustomWindow + 'static> DrawText for DrawWindow<CW> {
    fn text(&mut self, pass: Pass, pos: Vec2, col: Colour, text: &PreparedText) {
        // TODO: perhaps glyph_brush can accept an offset for all glyphs?
        let glyphs = text.positioned_glyphs(pos);
        let extra = (0..text.num_parts())
            .map(|_| Extra {
                color: col.into(),
                z: pass.depth(),
            })
            .collect();
        let min = pos.my_into();
        let max = (pos + text.bounds()).my_into();
        let bounds = ab_glyph::Rect { min, max };
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds);
    }

    fn text_glyph_pos(&mut self, ts: TextSection, index: usize) -> Vec2 {
        if index == 0 {
            // Short-cut. We also cannot iterate since there may be no glyphs.
            return ts.rect.pos.into();
        }
        let mut byte = None;
        let mut cum_len = 0;
        for part in ts.parts {
            let i = part.start as usize + index - cum_len;
            if i < part.end as usize {
                byte = Some(ts.text.as_bytes()[i]);
                break;
            };
            cum_len += part.len();
        }

        let pass = Pass::new_pass_with_depth(0, 0.0); // values are unimportant
        let mut iter = self.glyph_brush.glyphs(make_section(pass, &ts));

        let mut advance = false;
        let mut glyph;
        if let Some(byte) = byte {
            // Tiny HACK: line-breaks don't have glyphs
            if byte == b'\r' || byte == b'\n' {
                advance = true;
            }

            glyph = iter.next().unwrap().clone();
            for next in iter {
                if index < ts.parts[next.section_index].start as usize + next.byte_index {
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
        let scale = glyph.glyph.scale;
        let scale_font = PxScaleFont { font, scale };
        if advance {
            pos.x += scale_font.h_advance(glyph.glyph.id);
        }
        pos.y -= scale_font.ascent();
        return Vec2(pos.x, pos.y);
    }

    fn text_index_nearest(&mut self, ts: TextSection, pos: Vec2) -> usize {
        if ts.parts.len() == 0 {
            return 0; // short-cut
        }
        let text_len = ts.parts.iter().map(|part| part.len()).sum();
        // NOTE: if ts.parts.len() > 1 then base_to_mid may change, making the
        // row selection a little inaccurate. This method is best used with only
        // a single row of text anyway, so we consider this acceptable.
        // This also affects scale_font.h_advance at line-breaks. We consider
        // this a hack anyway and so tolerate some inaccuracy.
        let last_part = ts.parts.as_ref().last().unwrap();
        let scale_font = PxScaleFont {
            font: self.glyph_brush.fonts()[last_part.font.get()].clone(),
            scale: last_part.scale.my_into(),
        };
        let base_to_mid = -0.5 * scale_font.ascent();

        let pass = Pass::new_pass_with_depth(0, 0.0); // values are unimportant
        let mut iter = self.glyph_brush.glyphs(make_section(pass, &ts));

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
                    let index = last.byte_index;
                    let mut cursor = GraphemeCursor::new(index, text_len, true);
                    let mut cum_len = 0;
                    let text = 'outer: loop {
                        for part in ts.parts {
                            let len = part.len();
                            if index < cum_len + len {
                                break 'outer &ts.text[part.range()];
                            }
                            cum_len += len;
                        }
                        unreachable!();
                    };
                    let byte = cursor
                        .next_boundary(text, cum_len)
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
            best = (text_len, new_best);
        }

        assert!(
            best.0 <= text_len,
            "text_index_nearest: index beyond text length!"
        );
        best.0
    }
}
