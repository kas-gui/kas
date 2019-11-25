// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget styling
//!
//! Widget size and appearance can be modified through themes.
//!
//! Theme implementations depend on a graphics API (TODO).

use std::f32;
use std::marker::PhantomData;

use wgpu_glyph::{Font, HorizontalAlign, Layout, Scale, Section, VerticalAlign};

use kas::class::{Align, Class};
use kas::draw::*;
use kas::geom::{AxisInfo, Coord, Margins, Size, SizeRules};
use kas::{event, Widget};

use crate::draw::*;

/// A *theme* provides widget sizing and drawing implementations.
///
/// Objects of this type are copied within each window's data structure. For
/// large resources (e.g. fonts and icons) consider using external storage.
pub trait Theme: Clone {
    /// Implementor of draw API
    type Draw;

    /// Set the DPI factor.
    ///
    /// This method is called after constructing a window and each time the DPI
    /// changes (e.g. via system settings or with monitor-specific DPI factors).
    ///
    /// On "standard" monitors, the factor is 1. High-DPI screens may have a
    /// factor of 2 or higher. The factor may not be an integer; e.g.
    /// `9/8 = 1.125` works well with many 1440p screens. It is recommended to
    /// round dimensions to the nearest integer, and cache the result:
    /// ```notest
    /// self.margin = (MARGIN * factor).round();
    /// ```
    fn set_dpi_factor(&mut self, factor: f32);

    /// Get the list of available fonts
    ///
    /// Currently, all fonts used must be specified up front by this method.
    /// (Dynamic addition of fonts may be enabled in the future.)
    ///
    /// This is considered a "getter" rather than a "constructor" method since
    /// the `Font` type is cheap to copy, and each window requires its own copy.
    /// It may also be useful to retain a `Font` handle for access to its
    /// methods.
    ///
    /// Corresponding `FontId`s may be created from the index into this list.
    /// The first font in the list will be the default font.
    fn get_fonts<'a>(&self) -> Vec<Font<'a>>;

    /// Light source
    ///
    /// This affects shadows on frames, etc. The light source has neutral colour
    /// and intensity such that the colour of flat surfaces is unaffected.
    ///
    /// Return value: `(a, b)` where `0 ≤ a < pi/2` is the angle to the screen
    /// normal (i.e. `a = 0` is straight at the screen) and `b` is the bearing
    /// (from UP, clockwise), both in radians.
    ///
    /// Currently this is not updated after initial set-up.
    fn light_direction(&self) -> (f32, f32);

    /// Background colour
    fn clear_colour(&self) -> Colour;

    /// Margin and inter-row/column dimensions
    ///
    /// Margin dimensions are added to the area allocated to each widget. For
    /// simple widgets, margins may be specified here *or* by
    /// [`Theme::size_rules`]; for parent widgets, margins can only be specified
    /// by this method.
    fn margins(&self, widget: &dyn Widget) -> Margins;

    /// Widget dimensions
    ///
    /// Used to specify the dimension of a widget, based on class and contents.
    ///
    /// This method is *not* called on "parent" widgets (those with a layout
    /// other than "derive"); these widgets can only specify margins via the
    /// [`Theme::margins`] method.
    fn size_rules(&self, draw: &mut Self::Draw, widget: &dyn Widget, axis: AxisInfo) -> SizeRules;

    /// Draw a widget
    ///
    /// This method is called to draw each visible widget (and should not
    /// attempt recursion on child widgets).
    fn draw(&self, draw: &mut Self::Draw, ev_mgr: &event::Manager, widget: &dyn kas::Widget);
}

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

impl<D: DrawFlat + DrawSquare + DrawRound + DrawText> Theme for SampleTheme<D> {
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

    fn margins(&self, widget: &dyn Widget) -> Margins {
        match widget.class() {
            Class::Frame => {
                let off = self.frame_size as i32;
                let size = 2 * off as u32;
                Margins {
                    outer: Size(size, size),
                    offset: Coord(off, off),
                    inner: Coord::ZERO,
                }
            }
            _ => Margins {
                outer: Size::uniform(self.margin as u32 * 2),
                offset: Coord::ZERO,
                inner: Coord::ZERO,
            },
        }
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

        match widget.class() {
            Class::Container | Class::Frame | Class::Window => SizeRules::EMPTY, // not important
            Class::Label(_) => {
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    SizeRules::variable(line_height, bound(true).max(line_height))
                }
            }
            Class::Entry(_) => {
                let frame = 2 * self.frame_size as u32;
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min)) + frame
                } else {
                    SizeRules::fixed(line_height + frame)
                }
            }
            Class::Button(_) => {
                let f = 2 * self.button_frame as u32;
                if axis.horiz() {
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
        }
    }

    fn draw(&self, draw: &mut D, ev_mgr: &event::Manager, widget: &dyn kas::Widget) {
        // This is a hacky draw routine just to show where widgets are.
        let w_id = widget.id();

        // Note: coordinates place the origin at the top-left.
        let rect = widget.rect();
        let mut u = Vec2::from(rect.pos_f32());
        let size = Vec2::from(rect.size_f32());
        let mut v = u + size;

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
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                draw.draw_square_frame(s, t, u, v, (0.0, -0.8), FRAME);
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
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                draw.draw_round_frame(s, t, u, v, (0.0, 0.6), c);
                bounds = bounds - 2.0 * f;

                text = Some((cls.get_text(), BUTTON_TEXT));
            }
            Class::CheckBox(cls) => {
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                draw.draw_square_frame(s, t, u, v, (0.0, -0.8), FRAME);
                bounds = bounds - 2.0 * f;

                background = Some(TEXT_AREA);

                // TODO: draw check mark *and* optional text
                // let text = if cls.get_bool() { "✓" } else { "" };
                text = Some((cls.get_text(), TEXT));
            }
            Class::Frame => {
                draw.draw_round_frame(u, v, u + f, v - f, (0.6, -0.6), FRAME);
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
            let text_pos = u + margin + Vec2(h_offset, v_offset);

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

            let (s, t) = (u, v);
            u = u + margin;
            v = v - margin;
            draw.draw_square_frame(s, t, u, v, (0.0, 0.0), col);
        }

        if let Some(background) = background {
            draw.draw_flat_quad(u, v, background.into());
        }
    }
}
