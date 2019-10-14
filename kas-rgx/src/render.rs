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

    fn get_rect(&self, tkd: TkData) -> kas::Rect {
        self.ws[tkd.0 as usize].get_rect()
    }

    fn set_rect(&mut self, tkd: TkData, rect: &kas::Rect) {
        self.ws[tkd.0 as usize].set_rect(rect)
    }

    fn get_bool(&self, tkd: TkData) -> bool {
        self.ws[tkd.0 as usize].get_bool()
    }

    fn set_bool(&mut self, tkd: TkData, state: bool) {
        self.ws[tkd.0 as usize].set_bool(state)
    }

    fn set_text(&mut self, tkd: TkData, text: &str) {
        self.ws[tkd.0 as usize].set_text(text)
    }
}

trait Drawable: kas::Widget {}

struct Widget {
    rect: kas::Rect,
    details: WidgetDetails,
}

enum WidgetDetails {
    Container,
    Label(String),
    Entry(bool, String),
    Button(String),
    CheckBox(bool, String),
    Frame,
    Window,
}

impl Widget {
    #[inline]
    fn new(w: &mut dyn kas::Widget) -> Self {
        use kas::Class::*;
        Widget {
            rect: kas::Rect {
                pos: (0, 0),
                size: (0, 0),
            },
            details: match w.class() {
                Container => WidgetDetails::Container,
                Label(c) => WidgetDetails::Label(c.get_text().into()),
                Entry(c) => WidgetDetails::Entry(c.is_editable(), c.get_text().into()),
                Button(c) => WidgetDetails::Button(c.get_text().into()),
                CheckBox(c) => WidgetDetails::CheckBox(c.get_bool(), c.get_text().into()),
                Frame => WidgetDetails::Frame,
                Window => WidgetDetails::Window,
            },
        }
    }

    fn size_hints(&self) -> (Size, Size) {
        // FIXME
        let min = (20, 20);
        let hint = (80, 40);
        (min, hint)
    }

    fn get_rect(&self) -> kas::Rect {
        self.rect.clone()
    }

    fn set_rect(&mut self, rect: &kas::Rect) {
        self.rect = rect.clone();
    }

    fn get_bool(&self) -> bool {
        use WidgetDetails::*;
        match &self.details {
            Entry(b, ..) | CheckBox(b, ..) => *b,
            _ => panic!("Widget does not support get_bool!"),
        }
    }

    fn set_bool(&mut self, state: bool) {
        use WidgetDetails::*;
        match &mut self.details {
            Entry(b, ..) | CheckBox(b, ..) => *b = state,
            _ => panic!("Widget does not support set_bool!"),
        }
    }

    fn set_text(&mut self, text: &str) {
        use WidgetDetails::*;
        match &mut self.details {
            Label(s) | Entry(_, s) | Button(s) | CheckBox(_, s) => *s = text.into(),
            _ => panic!("Widget does not support set_text!"),
        }
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
        let (x0, y) = self.rect.pos_f32();
        let y1 = height - y;
        let (w, h) = self.rect.size_f32();
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
