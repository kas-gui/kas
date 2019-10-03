// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Backend widget code
//! 
//! TODO: most of this code would be completely unnecessary if we drop the
//! toolkit abstraction, which might make sense.

use rgx::core::*;
use rgx::kit::shape2d::{Pipeline, Batch, Fill, Line, Shape, Stroke};
use winit::event::WindowEvent;

use kas::{Coord, TkData, TkWidget};


pub(crate) struct Widgets {
    ws: Vec<Widget>,
}

impl Widgets {
    pub fn new() -> Self {
        Widgets { ws: vec![] }
    }
    
    pub fn add(&mut self, w: &mut dyn kas::Widget) {
        w.set_tkd(TkData(self.ws.len() as u64));
        
        self.ws.push(Widget::new(w));
        
        for i in 0..w.len() {
            self.add(w.get_mut(i).unwrap());
        }
    }
    
    pub fn ev_cursor_moved(&mut self, position: Coord) -> bool {
        // TODO: more efficient way of detecting hover
        let mut change = false;
        for w in &mut self.ws {
            let hover = w.rect.contains(position);
            if w.hover != hover {
                w.hover = hover;
                change = true;
            }
        }
        change
    }
    
    pub fn draw(&self, batch: &mut Batch, width: u32, height: u32) {
        /* This confirms our coordinate mapping
        let stroke = Stroke::new(1.0, Rgba::new(0.8, 0.8, 0.1, 1.0));
        let o = (0.0, 0.0);
        let t = (width as f32, height as f32);
        
        batch.add(Shape::Line(Line::new(o.0, o.1, o.0 + 5.0, o.1 + 5.0), stroke));
        batch.add(Shape::Line(Line::new(o.0, t.1, o.0 + 5.0, t.1 - 5.0), stroke));
        batch.add(Shape::Line(Line::new(t.0, o.1, t.0 - 5.0, o.1 + 5.0), stroke));
        batch.add(Shape::Line(Line::new(t.0, t.1, t.0 - 5.0, t.1 - 5.0), stroke));
        */
        
        let height = height as f32;
        for w in &self.ws {
            w.draw(batch, height);
        }
    }
}

impl TkWidget for Widgets {
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord) {
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


struct Widget {
    rect: kas::Rect,
    hover: bool,
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
            rect: kas::Rect { pos: (0, 0), size: (0, 0) },
            hover: false,
            details: match w.class() {
                Container => WidgetDetails::Container,
                Label(c) => WidgetDetails::Label(c.get_text().into()),
                Entry(c) => WidgetDetails::Entry(c.is_editable(), c.get_text().into()),
                Button(c) => WidgetDetails::Button(c.get_text().into()),
                CheckBox(c) => WidgetDetails::CheckBox(c.get_bool(), c.get_text().into()),
                Frame => WidgetDetails::Frame,
                Window => WidgetDetails::Window,
            }
        }
    }
    
    fn size_hints(&self) -> (Coord, Coord) {
        // FIXME
        let min = (10, 10);
        let hint = (50, 20);
        (min, hint)
    }
    
    fn get_rect(&self) -> kas::Rect {
        self.rect.clone()
    }
    
    fn set_rect(&mut self, rect: &kas::Rect) {
        println!("Rect: {:?}", rect);
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
    
    pub fn draw(&self, batch: &mut Batch, height: f32) {
        // Note: widget coordinates place the origin at the top-left.
        // Draw coordinates use f32 with the origin at the bottom-left.
        // Note: it's important to pass smallest coord to Shape::Rectangle first
        let (x0, y) = self.rect.pos_f32();
        let y1 = height - y;
        let (w, h) = self.rect.size_f32();
        let (x1, y0) = (x0 + w, y1 - h);
        
        let r = if self.hover { 1.0 } else { 0.5 };
        batch.add(Shape::Rectangle(
            Rect::new(x0, y0, x1, y1),
            Stroke::new(1.0, Rgba::new(r, 0.5, 0.5, 1.0)),
            Fill::Solid(Rgba::new(1.0, 1.0, 1.0, 0.1)),
        ));
    }
}
