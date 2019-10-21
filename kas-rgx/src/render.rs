// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use rgx::core::*;
use rgx::kit::shape2d::{Batch, Fill, Shape, Stroke};
use wgpu_glyph::{GlyphBrush, Scale, Section};

use kas::{Class, Size, SizePref, TkWidget, Widget, WidgetId};

/// Widget renderer
pub(crate) struct Widgets {
    hover: Option<WidgetId>,
    font_scale: f32,
    redraw: bool,
}

impl Widgets {
    pub fn new() -> Self {
        Widgets {
            hover: None,
            font_scale: 24.0,
            redraw: false,
        }
    }

    pub fn need_redraw(&self) -> bool {
        self.redraw
    }

    pub fn draw<'a, V>(
        &self,
        rend: &Renderer,
        glyph_brush: &mut GlyphBrush<'a, V>,
        size: (u32, u32),
        win: &dyn kas::Window,
    ) -> VertexBuffer {
        let mut batch = Batch::new();

        let height = size.1 as f32;
        self.draw_iter(&mut batch, glyph_brush, height, win.as_widget());

        batch.finish(rend)
    }

    fn draw_iter<'a, V>(
        &self,
        batch: &mut Batch,
        glyph_brush: &mut GlyphBrush<'a, V>,
        height: f32,
        widget: &dyn kas::Widget,
    ) {
        // draw widget; recurse over children
        self.draw_widget(batch, glyph_brush, height, widget);

        for n in 0..widget.len() {
            self.draw_iter(batch, glyph_brush, height, widget.get(n).unwrap());
        }
    }

    fn draw_widget<'a, V>(
        &self,
        batch: &mut Batch,
        glyph_brush: &mut GlyphBrush<'a, V>,
        height: f32,
        widget: &dyn kas::Widget,
    ) {
        // This is a hacky draw routine just to show where widgets are.

        // Note: widget coordinates place the origin at the top-left.
        // Draw coordinates use f32 with the origin at the bottom-left.
        // Note: it's important to pass smallest coord to Shape::Rectangle first
        let rect = widget.rect();
        let (x0, y) = rect.pos_f32();
        let y1 = height - y;
        let (w, h) = rect.size_f32();
        let (x1, y0) = (x0 + w, y1 - h);

        let mut background = Rgba::new(1.0, 1.0, 1.0, 0.1);

        let margin = 2.0;
        let text_pos = (x0 + margin, y + margin);
        let scale = Scale::uniform(self.font_scale);
        let bounds = (x1 - x0 - margin - margin, y1 - y0 - margin - margin);

        match widget.class() {
            Class::Container | Class::Window => {
                // do not draw containers
                return;
            }
            Class::Label(cls) => {
                let color = [1.0, 1.0, 1.0, 1.0];
                glyph_brush.queue(Section {
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
                glyph_brush.queue(Section {
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
                let color = [1.0, 1.0, 1.0, 1.0];
                glyph_brush.queue(Section {
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
                glyph_brush.queue(Section {
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
        let r = if Some(widget.number()) == self.hover {
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

impl TkWidget for Widgets {
    fn size_pref(&self, _widget: &dyn Widget, pref: SizePref) -> Size {
        if pref == SizePref::Min {
            Size::ZERO
        } else if pref == SizePref::Max {
            Size::MAX
        } else {
            Size(80, 40) // FIXME
        }
    }

    fn set_hover(&mut self, hover: Option<WidgetId>) {
        if self.hover != hover {
            self.hover = hover;
            self.redraw = true;
        }
    }
}
