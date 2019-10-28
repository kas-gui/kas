// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use std::f32;

use rgx::core::*;
use rgx::kit::shape2d::{Batch, Fill, Shape, Stroke};
use wgpu_glyph::{rusttype, GlyphBrush, GlyphCruncher, Scale, Section};

use kas::{Axes, Class, Size, SizePref, TkWidget, Widget, WidgetId};

const MARGIN: f32 = 2.0;

/// Widget renderer
pub(crate) struct Widgets {
    pub(crate) glyph_brush: GlyphBrush<'static, ()>,
    hover: Option<WidgetId>,
    click_start: Option<WidgetId>,
    font_scale: f32,
    redraw: bool,
}

impl Widgets {
    pub fn new(glyph_brush: GlyphBrush<'static, ()>) -> Self {
        Widgets {
            glyph_brush,
            hover: None,
            click_start: None,
            font_scale: 24.0,
            redraw: false,
        }
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

        let text_pos = (x0 + MARGIN, y + MARGIN);
        let scale = Scale::uniform(self.font_scale);
        let bounds = (x1 - x0 - MARGIN - MARGIN, y1 - y0 - MARGIN - MARGIN);

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
            Stroke::new(MARGIN, Rgba::new(r, 0.5, 0.5, 1.0)),
            Fill::Solid(background),
        ));
    }
}

impl TkWidget for Widgets {
    fn size_pref(&mut self, widget: &dyn Widget, pref: SizePref, axes: Axes) -> Size {
        // TODO: cache?
        let bounds = widget.class().text().and_then(|text| {
            let mut bounds = (f32::INFINITY, f32::INFINITY);
            if axes == Axes::Horiz(true) {
                bounds.1 = widget.rect().size.1 as f32;
            } else if axes == Axes::Vert(true) {
                bounds.0 = widget.rect().size.0 as f32;
            }
            self.glyph_brush.glyph_bounds(Section {
                text,
                screen_position: (0.0, 0.0),
                scale: Scale::uniform(self.font_scale),
                bounds,
                ..Section::default()
            })
        });
        let line_height = self.font_scale as u32;

        let size = match widget.class() {
            Class::Container | Class::Frame | Class::Window => Size(0, 0), // not important
            Class::Label(_) => {
                if pref < SizePref::Small {
                    Size(4 * line_height, line_height)
                } else if pref < SizePref::Max {
                    map_size_min(bounds, Size(0, line_height))
                } else {
                    Size::MAX
                }
            }
            Class::Entry(_) => {
                if pref < SizePref::Default {
                    Size(4 * line_height, line_height)
                } else if pref < SizePref::Max {
                    Size(8 * line_height, line_height)
                } else {
                    Size(Size::MAX.0, line_height)
                }
            }
            Class::Button(_) => {
                if pref < SizePref::Small {
                    Size(2 * line_height, line_height)
                } else if pref < SizePref::Large {
                    map_size_min(bounds, Size(line_height * 2, line_height))
                } else {
                    Size(Size::MAX.0, line_height)
                }
            }
            Class::CheckBox(_) => Size(line_height, line_height),
        };

        let margins = MARGIN as u32 * 2;
        size + Size(margins, margins)
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

fn map_size_min(rect: Option<rusttype::Rect<f32>>, ms: Size) -> Size {
    match rect {
        Some(rusttype::Rect { min, max }) => Size(
            ms.0.max((max.x - min.x) as u32),
            ms.1.max((max.y - min.y) as u32),
        ),
        None => ms,
    }
}
