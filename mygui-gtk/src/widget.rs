//! GTK backend
//! 
//! Widget code

use gtk;
use gtk::{WidgetExt};

use mygui::{Coord, Rect};
use mygui::toolkit::{TkData, TkWidget};

use super::GtkToolkit;
use super::tkd::borrow_from_tkd;


impl TkWidget for GtkToolkit {
    fn size_hints(&self, tkd: TkData) -> (Coord, Coord) {
        let wptr = unsafe { borrow_from_tkd(tkd) }.unwrap();
        let min = Coord::conv(wptr.get_preferred_size().0);
        let hint = Coord::conv(wptr.get_preferred_size().1);
        (min, hint)
    }
    
    fn set_rect(&self, tkd: TkData, rect: &Rect) {
        let wptr = unsafe { borrow_from_tkd(tkd) }.unwrap();
        let mut rect = gtk::Rectangle {
            x: rect.pos.0, y: rect.pos.1,
            width: rect.size.0, height: rect.size.1
        };
        wptr.size_allocate(&mut rect);
    }
}

// From, but constructed locally so that we can implement for foreign types
trait Convert<T> {
    fn conv(T) -> Self;
}

impl Convert<gtk::Requisition> for Coord {
    fn conv(rq: gtk::Requisition) -> Coord {
        (rq.width, rq.height)
    }
}
