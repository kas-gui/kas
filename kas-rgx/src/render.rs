// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;

use rgx::core::*;
use rgx::kit::shape2d::{Batch, Fill, Shape, Stroke};
use wgpu_glyph::{
    GlyphBrush, GlyphCruncher, HorizontalAlign, Layout, Scale, Section, VerticalAlign,
};

use kas::class::{Align, Class};
use kas::geom::{AxisInfo, SizeRules};
use kas::{event, TkAction, TkWindow, Widget};

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;

/// Widget renderer
pub(crate) struct Widgets {
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    font_scale: f32,
    margin: f32,
    action: TkAction,
    ev_mgr: event::ManagerData,
}

impl Widgets {
    pub fn new(dpi_factor: f64, glyph_brush: GlyphBrush<'static, ()>) -> Self {
        let dpi = dpi_factor as f32;
        Widgets {
            glyph_brush,
            font_scale: (FONT_SIZE * dpi).round(),
            margin: (MARGIN * dpi).round(),
            action: TkAction::None,
            ev_mgr: event::ManagerData::new(dpi_factor),
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f64) {
        let dpi = dpi_factor as f32;
        self.font_scale = (FONT_SIZE * dpi).round();
        self.margin = (MARGIN * dpi).round();
        self.ev_mgr.set_dpi_factor(dpi_factor);
        // Note: we rely on caller to resize widget
    }

    #[inline]
    pub fn pop_action(&mut self) -> TkAction {
        let action = self.action;
        self.action = TkAction::None;
        action
    }

    pub fn draw(
        &mut self,
        rend: &Renderer,
        size: (u32, u32),
        win: &dyn kas::Window,
    ) -> VertexBuffer {
        let mut batch = Batch::new();

        let height = size.1 as f32;
        self.draw_iter(&mut batch, height, win.as_widget());

        batch.finish(rend)
    }

    fn draw_iter(&mut self, batch: &mut Batch, height: f32, widget: &dyn kas::Widget) {
        // draw widget; recurse over children
        self.draw_widget(batch, height, widget);

        for n in 0..widget.len() {
            self.draw_iter(batch, height, widget.get(n).unwrap());
        }
    }

    fn draw_widget(&mut self, batch: &mut Batch, height: f32, widget: &dyn kas::Widget) {
        // This is a hacky draw routine just to show where widgets are.
        let w_id = widget.id();

        // Note: widget coordinates place the origin at the top-left.
        // Draw coordinates use f32 with the origin at the bottom-left.
        // Note: it's important to pass smallest coord to Shape::Rectangle first
        let rect = widget.rect();
        let (x0, y) = rect.pos_f32();
        let y1 = height - y;
        let (w, h) = rect.size_f32();
        let (x1, y0) = (x0 + w, y1 - h);

        let mut background = Rgba::new(1.0, 1.0, 1.0, 0.1);

        let margin = self.margin;
        let scale = Scale::uniform(self.font_scale);
        let bounds = (x1 - x0 - margin - margin, y1 - y0 - margin - margin);

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
        let text_pos = (x0 + margin + h_offset, y + margin + v_offset);

        match widget.class() {
            Class::Container | Class::Window => {
                // do not draw containers
                return;
            }
            Class::Label(cls) => {
                let color = [1.0, 1.0, 1.0, 1.0];
                let section = Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                };
                self.glyph_brush.queue(section);
            }
            Class::Entry(cls) => {
                background = Rgba::new(1.0, 1.0, 1.0, 1.0);
                let color = [0.0, 0.0, 0.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::Button(cls) => {
                background = Rgba::new(0.2, 0.7, 1.0, 1.0);
                if self.ev_mgr.is_depressed(w_id) {
                    background = Rgba::new(0.2, 0.6, 0.8, 1.0);
                } else if self.ev_mgr.is_hovered(w_id) {
                    background = Rgba::new(0.25, 0.8, 1.0, 1.0);
                }
                let color = [1.0, 1.0, 1.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::CheckBox(cls) => {
                background = Rgba::new(1.0, 1.0, 1.0, 1.0);
                let color = [0.0, 0.0, 0.0, 1.0];
                // TODO: draw check mark *and* optional text
                // let text = if cls.get_bool() { "✓" } else { "" };
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    layout,
                    ..Section::default()
                });
            }
            Class::Frame => {
                // TODO
            }
        }

        // draw margin
        let r = if self.ev_mgr.is_hovered(w_id) {
            1.0
        } else {
            0.5
        };
        batch.add(Shape::Rectangle(
            Rect::new(x0, y0, x1, y1),
            Stroke::new(margin, Rgba::new(r, 0.5, 0.5, 1.0)),
            Fill::Solid(background),
        ));
    }
}

impl TkWindow for Widgets {
    fn data(&self) -> &event::ManagerData {
        &self.ev_mgr
    }

    fn update_data(&mut self, f: &dyn Fn(&mut event::ManagerData) -> bool) {
        if f(&mut self.ev_mgr) {
            self.send_action(TkAction::Redraw);
        }
    }

    fn size_rules(&mut self, widget: &dyn Widget, axis: AxisInfo) -> SizeRules {
        let line_height = self.font_scale as u32;
        let mut bound = |vert: bool| -> u32 {
            let bounds = widget.class().text().and_then(|text| {
                let mut bounds = (f32::INFINITY, f32::INFINITY);
                if let Some(size) = axis.fixed(false) {
                    bounds.1 = size as f32;
                } else if let Some(size) = axis.fixed(true) {
                    bounds.0 = size as f32;
                }
                self.glyph_brush.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(self.font_scale),
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
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    SizeRules::variable(line_height, bound(true).max(line_height))
                }
            }
            Class::Entry(_) | Class::Button(_) => {
                if axis.horiz() {
                    let min = 3 * line_height;
                    SizeRules::variable(min, bound(false).max(min))
                } else {
                    SizeRules::fixed(line_height)
                }
            }
            Class::CheckBox(_) => SizeRules::fixed(line_height),
        };

        size + SizeRules::fixed(self.margin as u32 * 2)
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
