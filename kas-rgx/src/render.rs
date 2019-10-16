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
    ws: Vec<Widget>,
    pub(crate) hover: Option<WidgetId>,
}

impl Widgets {
    pub fn new() -> Self {
        Widgets {
            ws: vec![],
            hover: None,
        }
    }

    pub fn add(&mut self, w: &mut dyn kas::Widget) {
        w.set_tkd(TkData(self.ws.len() as u64));

        self.ws.push(Widget::new(w));

        for i in 0..w.len() {
            self.add(w.get_mut(i).unwrap());
        }
    }

    pub fn draw(&self, rend: &Renderer, size: (u32, u32), win: &dyn kas::Window) -> VertexBuffer {
        let mut batch = Batch::new();

        let height = size.1 as f32;
        self.draw_widget(&mut batch, height, win.as_widget());

        batch.finish(rend)
    }

    fn draw_widget(&self, batch: &mut Batch, height: f32, w: &dyn kas::Widget) {
        // draw widget; recurse over children
        let n = w.tkd().0 as usize;
        self.ws[n].draw(self, batch, height, w);

        for n in 0..w.len() {
            self.draw_widget(batch, height, w.get(n).unwrap());
        }
    }
}

impl TkWidget for Widgets {
    fn size_hints(&self, tkd: TkData) -> (Size, Size) {
        self.ws[tkd.0 as usize].size_hints()
    }
}

trait Drawable: kas::Widget {}

struct Widget {
}

impl Widget {
    #[inline]
    fn new(_: &mut dyn kas::Widget) -> Self {
        Widget {}
    }

    fn size_hints(&self) -> (Size, Size) {
        // FIXME
        let min = Size(20, 20);
        let hint = Size(80, 40);
        (min, hint)
    }

    pub fn draw(
        &self,
        widgets: &Widgets,
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
        let r = if Some(widget.number()) == widgets.hover {
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
