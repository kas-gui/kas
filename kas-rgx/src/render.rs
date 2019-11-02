// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;

use rgx::core::*;
use rgx::kit::shape2d::{Batch, Fill, Shape, Stroke};
use wgpu_glyph::{GlyphBrush, GlyphCruncher, Scale, Section};

use kas::{Align, AxisInfo, Class, SizeRules, TkWidget, Widget, WidgetId};

/// Font size (units are half-point sizes?)
const FONT_SIZE: f32 = 20.0;
/// Inner margin; this is multiplied by the DPI factor then rounded to nearest
/// integer, e.g. `(2.0 * 1.25).round() == 3.0`.
const MARGIN: f32 = 2.0;

/// Widget renderer
pub(crate) struct Widgets {
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    hover: Option<WidgetId>,
    click_start: Option<WidgetId>,
    font_scale: f32,
    margin: f32,
    redraw: bool,
}

impl Widgets {
    pub fn new(dpi: f32, glyph_brush: GlyphBrush<'static, ()>) -> Self {
        Widgets {
            glyph_brush,
            hover: None,
            click_start: None,
            font_scale: (FONT_SIZE * dpi).round(),
            margin: (MARGIN * dpi).round(),
            redraw: false,
        }
    }

    pub fn set_dpi_factor(&mut self, dpi: f32) {
        self.font_scale = (FONT_SIZE * dpi).round();
        self.margin = (MARGIN * dpi).round();
        // Note: we rely on caller to resize widget
    }

    pub fn need_redraw(&self) -> bool {
        self.redraw
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
        let w_id = Some(widget.number());

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

        // TODO: can we cache this, yet update it whenever contents change?
        // Currently we rely on glyph_brush's caching.
        let text_bounds = widget
            .class()
            .text()
            .and_then(|text| {
                self.glyph_brush.glyph_bounds(Section {
                    text,
                    screen_position: (0.0, 0.0),
                    scale: Scale::uniform(self.font_scale),
                    bounds,
                    ..Section::default()
                })
            })
            .map(|rect| (rect.max.x - rect.min.x, rect.max.y - rect.min.y))
            .unwrap_or((0.0, 0.0));

        let alignments = widget.class().alignments();
        // TODO: support justified alignment
        let woffset = match alignments.1 {
            Align::Begin | Align::Justify => 0.0,
            Align::Center => 0.5 * (bounds.0 - text_bounds.0 as f32),
            Align::End => bounds.0 - text_bounds.0 as f32,
        };
        let hoffset = match alignments.1 {
            Align::Begin | Align::Justify => 0.0,
            Align::Center => 0.5 * (bounds.1 - text_bounds.1 as f32),
            Align::End => bounds.1 - text_bounds.1 as f32,
        };
        let text_pos = (x0 + margin + woffset, y + margin + hoffset);

        match widget.class() {
            Class::Container | Class::Window => {
                // do not draw containers
                return;
            }
            Class::Label(cls) => {
                let color = [1.0, 1.0, 1.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
                    ..Section::default()
                });
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
                    ..Section::default()
                });
            }
            Class::Button(cls) => {
                background = Rgba::new(0.2, 0.7, 1.0, 1.0);
                if self.hover == w_id {
                    if self.click_start == w_id {
                        background = Rgba::new(0.2, 0.6, 0.8, 1.0);
                    } else if self.click_start.is_none() {
                        background = Rgba::new(0.25, 0.8, 1.0, 1.0);
                    }
                }
                let color = [1.0, 1.0, 1.0, 1.0];
                self.glyph_brush.queue(Section {
                    text: cls.get_text(),
                    screen_position: text_pos,
                    color,
                    scale,
                    bounds,
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
                    ..Section::default()
                });
            }
            Class::Frame => {
                // TODO
            }
        }

        // draw margin
        let r = if self.hover == w_id { 1.0 } else { 0.5 };
        batch.add(Shape::Rectangle(
            Rect::new(x0, y0, x1, y1),
            Stroke::new(margin, Rgba::new(r, 0.5, 0.5, 1.0)),
            Fill::Solid(background),
        ));
    }
}

impl TkWidget for Widgets {
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

    fn redraw(&mut self, _: &dyn Widget) {
        self.redraw = true;
    }

    fn hover(&self) -> Option<WidgetId> {
        self.hover
    }
    fn set_hover(&mut self, id: Option<WidgetId>) {
        if self.hover != id {
            self.hover = id;
            self.redraw = true;
        }
    }

    fn click_start(&self) -> Option<WidgetId> {
        self.click_start
    }
    fn set_click_start(&mut self, id: Option<WidgetId>) {
        self.click_start = id;
    }
}
