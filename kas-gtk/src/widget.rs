// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! GTK backend
//! 
//! Widget code

use gtk;
use gtk::{Cast, WidgetExt, LabelExt, ButtonExt, EntryExt, ToggleButtonExt};

use kas::{Coord, Rect, TkData, TkWidget};

use super::tkd::borrow_from_tkd;


pub struct Toolkit;

impl TkWidget for Toolkit {
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord) {
        let gw = unsafe { borrow_from_tkd(tkd) }.unwrap();
        let min = Coord::conv(gw.get_preferred_size().0);
        let hint = Coord::conv(gw.get_preferred_size().1);
        (min, hint)
    }
    
    fn get_rect(&self, tkd: TkData) -> Rect {
        let gw = unsafe { borrow_from_tkd(tkd) }.unwrap();
        Rect::conv(gw.get_allocation())
    }
    
    fn set_rect(&self, tkd: TkData, rect: &Rect) {
        let gw = unsafe { borrow_from_tkd(tkd) }.unwrap();
        let mut rect = gtk::Rectangle::conv(rect);
        gw.size_allocate(&mut rect);
    }
    
    fn set_bool(&self, tkd: TkData, state: bool) {
        let gw = unsafe { borrow_from_tkd(tkd) }.unwrap();
        if let Some(b) = gw.downcast_ref::<gtk::CheckButton>() {
            b.set_active(state);
        }
    }
    
    fn set_text(&self, tkd: TkData, text: &str) {
        let gw = unsafe { borrow_from_tkd(tkd) }.unwrap();
        if let Some(glabel) = gw.downcast_ref::<gtk::Label>() {
            glabel.set_label(text);
        } else if let Some(button) = gw.downcast_ref::<gtk::Button>() {
            button.set_label(text);
        } else if let Some(entry) = gw.downcast_ref::<gtk::Entry>() {
            entry.set_text(text);
        } /*else if let Some(cont) = gw.downcast_ref::<gtk::Container>() {
            // GTK sometimes uses a child for the actual label
            // TODO: consider using child_notify instead
            for child in cont.get_children().iter() {
                if let Some(glabel) = child.downcast_ref::<gtk::Label>() {
                    glabel.set_label(text);
                    break;
                }
            }
        }*/
    }
}

// From, but constructed locally so that we can implement for foreign types
trait Convert<T> {
    fn conv(t: T) -> Self;
}

impl Convert<gtk::Requisition> for Coord {
    fn conv(rq: gtk::Requisition) -> Self {
        (rq.width, rq.height)
    }
}

impl Convert<gtk::Rectangle> for Rect {
    fn conv(rect: gtk::Rectangle) -> Self {
        Rect {
            pos: (rect.x, rect.y),
            size: (rect.width, rect.height)
        }
    }
}

impl<'a> Convert<&'a Rect> for gtk::Rectangle {
    fn conv(rect: &'a Rect) -> Self {
        gtk::Rectangle {
            x: rect.pos.0, y: rect.pos.1,
            width: rect.size.0, height: rect.size.1
        }
    }
}
