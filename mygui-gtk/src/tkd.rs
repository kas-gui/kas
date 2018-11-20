//! GTK backend
//! 
//! ToolkitData utilities

use std::mem::transmute;

use gtk;

use mygui::{Core, TkData};


/// Abstractions over mygui::Widget
pub(crate) trait WidgetAbstraction {
    /// Store a GTK widget pointer in the TKD
    /// 
    /// The widget pointer must out-live the TKD, but since GTK widgets are
    /// heap-allocated and use reference-counted pointers, this is given. There
    /// is a possibility of memory-leak if `clear_gw` is not used to clear TKD.
    fn set_gw<'a, 'b>(&'a mut self, gw: &'b gtk::Widget);
    
    /// Clear the TKD, decrementing any stored pointer reference.
    fn clear_gw(&mut self);
    
    /// Get a reference-counted pointer to the widget stored.
    fn get_gw(&self) -> Option<gtk::Widget>;
    
    /// Get a borrowing pointer to the widget, if stored.
    /// 
    /// This borrowing reference does not increment reference counts, hence it
    /// is the responsibility of the programmer to ensure the underlying widget
    /// outlives the returned pointer. If in doubt use `get_gw`.
    unsafe fn borrow_gw(&self) -> Option<gtk::Widget>;
}

impl<T: Core + ?Sized> WidgetAbstraction for T {
    fn set_gw<'a, 'b>(&'a mut self, gw: &'b gtk::Widget) {
        self.clear_gw();
        self.set_tkd(unsafe { own_to_tkd(gw) });
    }
    
    fn clear_gw(&mut self) {
        // convert back to smart pointer to reduce reference count
        if let Some(_) = unsafe { own_from_tkd(self.tkd()) } {
            // mark empty
            self.set_tkd(Default::default());
        }
    }
    
    fn get_gw(&self) -> Option<gtk::Widget> {
        unsafe { ref_from_tkd(self.tkd()) }
    }
    
    unsafe fn borrow_gw(&self) -> Option<gtk::Widget> {
        borrow_from_tkd(self.tkd())
    }
}


pub(crate) unsafe fn own_to_tkd(w: &gtk::Widget) -> TkData {
    use glib::translate::ToGlibPtr;
    let ptr = gtk::Widget::to_glib_full(w);
    let mut tkd = TkData::default();
    tkd.0 = transmute::<*mut ::gtk_sys::GtkWidget, u64>(ptr);
    tkd
}

pub(crate) unsafe fn own_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrFull;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_full(ptr))
    }
}

pub(crate) unsafe fn ref_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrNone;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_none(ptr))
    }
}

pub(crate) unsafe fn borrow_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrBorrow;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_borrow(ptr))
    }
}
