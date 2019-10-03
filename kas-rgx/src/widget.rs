// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Backend widget code
//! 
//! TODO: most of this code would be completely unnecessary if we drop the
//! toolkit abstraction, which might make sense.

use kas::{Coord, Rect, TkData, TkWidget};


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
}

impl TkWidget for Widgets {
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord) {
        self.ws[tkd.0 as usize].size_hints()
    }
    
    fn get_rect(&self, tkd: TkData) -> Rect {
        self.ws[tkd.0 as usize].get_rect()
    }
    
    fn set_rect(&mut self, tkd: TkData, rect: &Rect) {
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
    rect: Rect,
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
            rect: Rect { pos: (0, 0), size: (0, 0) },
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
    
    fn get_rect(&self) -> Rect {
        self.rect.clone()
    }
    
    fn set_rect(&mut self, rect: &Rect) {
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
}
