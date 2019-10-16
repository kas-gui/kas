// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Widget rendering

use rgx::core::*;
use rgx::kit::shape2d::{Batch, Fill, Shape, Stroke};

use kas::{Class, Size, TkData, TkWidget, WidgetId};

// TODO: we can probably remove the ws field entirely, along with
// most TkWidget methods
pub(crate) struct Widgets {
    pub(crate) hover: Option<WidgetId>,
}

impl Widgets {
    pub fn new() -> Self {
        Widgets {
            hover: None,
        }
    }

    pub fn draw(&self, rend: &Renderer, size: (u32, u32), win: &dyn kas::Window) -> VertexBuffer {
        let mut batch = Batch::new();

        let height = size.1 as f32;
        self.draw_iter(&mut batch, height, win.as_widget());

        batch.finish(rend)
    }

    fn draw_iter(&self, batch: &mut Batch, height: f32, widget: &dyn kas::Widget) {
        // draw widget; recurse over children
        self.draw_widget(batch, height, widget);

        for n in 0..widget.len() {
            self.draw_iter(batch, height, widget.get(n).unwrap());
        }
    }
    
    fn draw_widget(
        self: &Widgets,
        batch: &mut Batch,
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

        match widget.class() {
            Class::Container => {
                // do not draw containers
                return;
            }
            Class::Button(_) => {
                background = Rgba::new(0.2, 0.7, 1.0, 1.0);
            }
            _ => (),
        }

        // draw margin
        let r = if Some(widget.number()) == self.hover {
            1.0
        } else {
            0.5
        };
        batch.add(Shape::Rectangle(
            Rect::new(x0, y0, x1, y1),
            Stroke::new(2.0, Rgba::new(r, 0.5, 0.5, 1.0)),
            Fill::Solid(background),
        ));
    }
}

impl TkWidget for Widgets {
    fn size_hints(&self, _: TkData) -> (Size, Size) {
        // FIXME
        let min = Size(20, 20);
        let hint = Size(80, 40);
        (min, hint)
    }
}
