// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.

use std::f32;
use std::marker::PhantomData;

use wgpu_glyph::{Font, HorizontalAlign, Layout, Scale, Section, VerticalAlign};

use kas::class::{Align, Class};
use kas::draw::*;
use kas::geom::Size;
use kas::layout::{AxisInfo, SizeRules};
use kas::{event, Widget};

use crate::draw::*;

/// A simple, inflexible theme providing a sample implementation.
#[derive(Copy, Debug, Default)]
pub struct SampleTheme<D> {
    font_scale: f32,
    margin: f32,
    frame_size: f32,
    button_frame: f32,
    _phantom: PhantomData<D>,
}

impl<D> SampleTheme<D> {
    /// Construct
    pub fn new() -> Self {
        SampleTheme {
            font_scale: FONT_SIZE,
            margin: MARGIN,
            frame_size: FRAME_SIZE,
            button_frame: BUTTON_FRAME,
            _phantom: Default::default(),
        }
    }
}

// manual impl because derive macro applies incorrect bounds
impl<D> Clone for SampleTheme<D> {
    fn clone(&self) -> Self {
        Self {
            font_scale: self.font_scale,
            margin: self.margin,
            frame_size: self.frame_size,
            button_frame: self.button_frame,
            _phantom: Default::default(),
        }
    }
}

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;
/// Frame size (adjusted as above)
const FRAME_SIZE: f32 = 5.0;
/// Button frame size (non-flat outer region)
const BUTTON_FRAME: f32 = 5.0;

/// Background colour
pub const BACKGROUND: Colour = Colour::grey(0.7);
/// Frame colour
pub const FRAME: Colour = BACKGROUND;
/// Text background
pub const TEXT_AREA: Colour = Colour::grey(1.0);

/// Text in text area
pub const TEXT: Colour = Colour::grey(0.0);
/// Text on background
pub const LABEL_TEXT: Colour = Colour::grey(0.0);
/// Text on button
pub const BUTTON_TEXT: Colour = Colour::grey(1.0);

impl<D: Draw + DrawText> Theme for SampleTheme<D> {
    type Draw = D;

    fn set_dpi_factor(&mut self, factor: f32) {
        self.font_scale = (FONT_SIZE * factor).round();
        self.margin = (MARGIN * factor).round();
        self.frame_size = (FRAME_SIZE * factor).round();
        self.button_frame = (BUTTON_FRAME * factor).round();
    }

    fn get_fonts<'a>(&self) -> Vec<Font<'a>> {
        vec![crate::font::get_font()]
    }

    fn light_direction(&self) -> (f32, f32) {
        (0.3, 0.4)
    }

    fn clear_colour(&self) -> Colour {
        BACKGROUND
    }

    fn size_rules(&self, draw: &mut D, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        let font_scale = self.font_scale;
        let line_height = font_scale as u32;
        let mut bound = |vert: bool| -> u32 {
            let bounds = widget.class().text().and_then(|text| {
                let mut bounds = (f32::INFINITY, f32::INFINITY);
                if let Some(size) = axis.fixed(false) {
                    bounds.1 = size as f32;
                } else if let Some(size) = axis.fixed(true) {
                    bounds.0 = size as f32;
                }
                draw.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(font_scale),
                    bounds,
                    ..Section::default()
                })
            });

            bounds
                .map(|(min, max)| match vert {
                    false => (max - min).0,
                    true => (max - min).1,
                } as u32)
                .unwrap_or(0)
        };

        let inner = match widget.class() {
            Class::Frame => return SizeRules::fixed(self.frame_size as u32),
            Class::Container | Class::Window => return SizeRules::EMPTY,
            Class::Label(_) => {
                if !axis.vertical() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    SizeRules::variable(line_height, bound(true).max(line_height))
                }
            }
            Class::Entry(_) => {
                let frame = 2 * self.frame_size as u32;
                if !axis.vertical() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min)) + frame
                } else {
                    SizeRules::fixed(line_height + frame)
                }
            }
            Class::Button(_) => {
                let f = 2 * self.button_frame as u32;
                if !axis.vertical() {
                    let min = 3 * line_height + f;
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    SizeRules::fixed(line_height + f)
                }
            }
            Class::CheckBox(_) => {
                let frame = 2 * self.frame_size as u32;
                SizeRules::fixed(line_height + frame)
            }
        };
        let margin = SizeRules::fixed(2 * self.margin as u32);
        inner + margin
    }

    fn inner_margin(&self, _: &dyn Widget, _: bool) -> u32 {
        0
    }

    fn child_margins(&self, widget: &dyn Widget) -> (Size, Size, Size) {
        match widget.class() {
            Class::Frame => {
                let f = self.frame_size as u32;
                (Size::uniform(f), Size::ZERO, Size::uniform(f))
            }
            _ => (Size::ZERO, Size::ZERO, Size::ZERO),
        }
    }

    fn draw(&self, draw: &mut D, ev_mgr: &event::Manager, widget: &dyn kas::Widget) {
        // This is a hacky draw routine just to show where widgets are.
        let w_id = widget.id();

        // Note: coordinates place the origin at the top-left.
        let rect = widget.rect();
        let p = Vec2::from(rect.pos_f32());
        let size = Vec2::from(rect.size_f32());
        let mut quad = Quad(p, p + size);

        let mut background = None;

        let margin = self.margin;
        let scale = Scale::uniform(self.font_scale);
        let mut bounds = size - 2.0 * margin;

        let f = self.frame_size;

        let mut _string; // Entry needs this to give a valid lifetime
        let text: Option<(&str, Colour)>;

        match widget.class() {
            Class::Container | Class::Window => {
                // do not draw containers
                return;
            }
            Class::Label(cls) => {
                text = Some((cls.get_text(), LABEL_TEXT));
            }
            Class::Entry(cls) => {
                let outer = quad;
                quad.shrink(f);
                let style = Style::Square(Vec2(0.0, -0.8));
                draw.draw_frame(outer, quad, style, FRAME);
                bounds = bounds - 2.0 * f;

                background = Some(TEXT_AREA);

                _string = cls.get_text().to_string();
                if ev_mgr.key_grab(w_id) {
                    // TODO: proper edit character and positioning
                    _string.push('|');
                }
                text = Some((&_string, TEXT));
            }
            Class::Button(cls) => {
                let c = if ev_mgr.is_depressed(w_id) {
                    Colour::new(0.2, 0.6, 0.8)
                } else if ev_mgr.is_hovered(w_id) {
                    Colour::new(0.25, 0.8, 1.0)
                } else {
                    Colour::new(0.2, 0.7, 1.0)
                };
                background = Some(c);

                let f = self.button_frame;
                let outer = quad;
                quad.shrink(f);
                let style = Style::Round(Vec2(0.0, 0.6));
                draw.draw_frame(outer, quad, style, c);
                bounds = bounds - 2.0 * f;

                text = Some((cls.get_text(), BUTTON_TEXT));
            }
            Class::CheckBox(cls) => {
                let outer = quad;
                quad.shrink(f);
                let style = Style::Square(Vec2(0.0, -0.8));
                draw.draw_frame(outer, quad, style, FRAME);
                bounds = bounds - 2.0 * f;

                background = Some(TEXT_AREA);

                // TODO: draw check mark *and* optional text
                // let text = if cls.get_bool() { "✓" } else { "" };
                text = Some((cls.get_text(), TEXT));
            }
            Class::Frame => {
                let outer = quad;
                quad.shrink(f);
                let style = Style::Round(Vec2(0.6, -0.6));
                draw.draw_frame(outer, quad, style, FRAME);
                return;
            }
        }

        if let Some((text, colour)) = text {
            let alignments = widget.class().alignments();
            // TODO: support justified alignment
            let (h_align, h_offset) = match alignments.1 {
                Align::Begin | Align::Justify => (HorizontalAlign::Left, 0.0),
                Align::Center => (HorizontalAlign::Center, 0.5 * bounds.0),
                Align::End => (HorizontalAlign::Right, bounds.0),
            };
            let (v_align, v_offset) = match alignments.1 {
                Align::Begin | Align::Justify => (VerticalAlign::Top, 0.0),
                Align::Center => (VerticalAlign::Center, 0.5 * bounds.1),
                Align::End => (VerticalAlign::Bottom, bounds.1),
            };
            let layout = Layout::default().h_align(h_align).v_align(v_align);
            let text_pos = quad.0 + margin + Vec2(h_offset, v_offset);

            draw.draw_text(Section {
                text,
                screen_position: text_pos.into(),
                color: colour.into(),
                scale,
                bounds: bounds.into(),
                layout,
                ..Section::default()
            });
        }

        // draw any highlights within the margin area
        // note: may be disabled via 'false &&' prefix
        let hover = false && ev_mgr.is_hovered(w_id);
        let key_focus = ev_mgr.key_focus(w_id);
        let key_grab = ev_mgr.key_grab(w_id);
        if hover || key_focus || key_grab {
            let mut col = Colour::new(0.7, 0.7, 0.7);
            if hover {
                col.g = 1.0;
            }
            if key_focus {
                col.r = 1.0;
            }
            if key_grab {
                col.b = 1.0;
            }

            let outer = quad;
            quad.shrink(margin);
            let style = Style::Flat;
            draw.draw_frame(outer, quad, style, col);
        }

        if let Some(background) = background {
            draw.draw_quad(quad, Style::Flat, background.into());
        }
    }
}
