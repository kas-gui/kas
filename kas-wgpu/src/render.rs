// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;

use wgpu_glyph::{
    GlyphBrush, GlyphCruncher, HorizontalAlign, Layout, Scale, Section, VerticalAlign,
};

use kas::class::{Align, Class};
use kas::geom::{AxisInfo, Coord, Margins, Size, SizeRules};
use kas::{event, TkAction, TkWindow, Widget};

use crate::colour::{self, Colour};
use crate::round_pipe::Rounded;
use crate::tri_pipe::TriPipe;
use crate::vertex::Vec2;

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;
/// Frame size (adjusted as above)
const FRAME_SIZE: f32 = 4.0;
/// Button frame size (non-flat outer region)
const BUTTON_FRAME: f32 = 6.0;

/// Widget renderer
pub(crate) struct Widgets {
    round_pipe: Rounded,
    tri_pipe: TriPipe,
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    font_scale: f32,
    margin: f32,
    frame_size: f32,
    button_frame: f32,
    action: TkAction,
    pub(crate) ev_mgr: event::ManagerData,
}

impl Widgets {
    pub fn new(
        device: &wgpu::Device,
        size: Size,
        dpi_factor: f64,
        glyph_brush: GlyphBrush<'static, ()>,
    ) -> Self {
        let round_pipe = Rounded::new(device, size);
        let tri_pipe = TriPipe::new(device, size);

        let dpi = dpi_factor as f32;
        Widgets {
            round_pipe,
            tri_pipe,
            glyph_brush,
            font_scale: (FONT_SIZE * dpi).round(),
            margin: (MARGIN * dpi).round(),
            frame_size: (FRAME_SIZE * dpi).round(),
            button_frame: (BUTTON_FRAME * dpi).round(),
            action: TkAction::None,
            ev_mgr: event::ManagerData::new(dpi_factor),
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        let dpi = dpi_factor as f32;
        self.font_scale = (FONT_SIZE * dpi).round();
        self.margin = (MARGIN * dpi).round();
        self.frame_size = (FRAME_SIZE * dpi).round();
        self.button_frame = (BUTTON_FRAME * dpi).round();
        self.ev_mgr.set_dpi_factor(dpi_factor);
        // Note: we rely on caller to resize widget
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: Size) -> wgpu::CommandBuffer {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.tri_pipe.resize(device, &mut encoder, size);
        self.round_pipe.resize(device, &mut encoder, size);
        encoder.finish()
    }

    #[inline]
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        rpass: &mut wgpu::RenderPass,
        win: &dyn kas::Window,
    ) {
        self.draw_iter(win.as_widget());
        self.tri_pipe.render(device, rpass);
        self.round_pipe.render(device, rpass);
    }

    fn draw_iter(&mut self, widget: &dyn kas::Widget) {
        // draw widget; recurse over children
        self.draw_widget(widget);

        for n in 0..widget.len() {
            self.draw_iter(widget.get(n).unwrap());
        }
    }

    fn draw_widget(&mut self, widget: &dyn kas::Widget) {
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
                text = Some((cls.get_text(), colour::LABEL_TEXT));
            }
            Class::Entry(cls) => {
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                self.tri_pipe
                    .add_frame(s, t, u, v, (0.0, 0.8), colour::FRAME);
                bounds = bounds - 2.0 * f;

                background = Some(colour::TEXT_AREA);

                _string = cls.get_text().to_string();
                if self.ev_mgr.key_grab(w_id) {
                    // TODO: proper edit character and positioning
                    _string.push('|');
                }
                text = Some((&_string, colour::TEXT));
            }
            Class::Button(cls) => {
                let c = if self.ev_mgr.is_depressed(w_id) {
                    Colour::new(0.2, 0.6, 0.8)
                } else if self.ev_mgr.is_hovered(w_id) {
                    Colour::new(0.25, 0.8, 1.0)
                } else {
                    Colour::new(0.2, 0.7, 1.0)
                };
                background = Some(c);

                let f = self.button_frame;
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                self.round_pipe.add_frame(s, t, u, v, c);
                bounds = bounds - 2.0 * f;

                text = Some((cls.get_text(), colour::BUTTON_TEXT));
            }
            Class::CheckBox(cls) => {
                let (s, t) = (u, v);
                u = u + f;
                v = v - f;
                self.tri_pipe
                    .add_frame(s, t, u, v, (0.0, 0.8), colour::FRAME);
                bounds = bounds - 2.0 * f;

                background = Some(colour::TEXT_AREA);

                // TODO: draw check mark *and* optional text
                // let text = if cls.get_bool() { "✓" } else { "" };
                text = Some((cls.get_text(), colour::TEXT));
            }
            Class::Frame => {
                self.tri_pipe
                    .add_frame(u, v, u + f, v - f, (0.0, 0.8), colour::FRAME);
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

            self.glyph_brush.queue(Section {
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
        let hover = false && self.ev_mgr.is_hovered(w_id);
        let key_focus = self.ev_mgr.key_focus(w_id);
        let key_grab = self.ev_mgr.key_grab(w_id);
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
            self.tri_pipe.add_frame(s, t, u, v, (0.0, 0.0), col);
        }

        if let Some(background) = background {
            self.tri_pipe.add_quad(u, v, background.into());
        }
    }
}

impl TkWindow for Widgets {
    fn data(&self) -> &event::ManagerData {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &mut dyn FnMut(&mut event::ManagerData) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        let line_height = self.font_scale as u32;
        let bound = |s: &mut Widgets, vert: bool| -> u32 {
            let bounds = widget.class().text().and_then(|text| {
                let mut bounds = (f32::INFINITY, f32::INFINITY);
                if let Some(size) = axis.fixed(false) {
                    bounds.1 = size as f32;
                } else if let Some(size) = axis.fixed(true) {
                    bounds.0 = size as f32;
                }
                s.glyph_brush.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(s.font_scale),
                    bounds,
                    ..Section::default()
                })
            });

            bounds
                .map(|rect| match vert {
                    false => rect.max.x - rect.min.x,
                    true => rect.max.y - rect.min.y,
                } as u32)
                .unwrap_or(0)
        };

        let size = match widget.class() {
            Class::Container | Class::Frame | Class::Window => SizeRules::EMPTY, // not important
            Class::Label(_) => {
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(self, false).max(min))
                } else {
                    SizeRules::variable(line_height, bound(self, true).max(line_height))
                }
            }
            Class::Entry(_) => {
                let frame = 2 * self.frame_size as u32;
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(self, false).max(min)) + frame
                } else {
                    SizeRules::fixed(line_height + frame)
                }
            }
            Class::Button(_) => {
                let f = 2 * self.button_frame as u32;
                if axis.horiz() {
                    let min = 3 * line_height + f;
                    SizeRules::variable(min, bound(self, false).max(min))
                } else {
                    SizeRules::fixed(line_height + f)
                }
            }
            Class::CheckBox(_) => {
                let frame = 2 * self.frame_size as u32;
                SizeRules::fixed(line_height + frame)
            }
        };

        size + SizeRules::fixed(self.margin as u32 * 2)
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
                outer: Size::ZERO,
                offset: Coord::ZERO,
                inner: Coord::ZERO,
            },
        }
    }

    #[inline]
    fn redraw(&mut self, _: &dyn Widget) {
        self.send_action(TkAction::Redraw);
    }

    #[inline]
    fn send_action(&mut self, action: TkAction) {
        self.action = self.action.max(action);
    }
}
