//! GTK backend
//! 
//! ToolkitData utilities

use std::mem::transmute;

use gtk;

use toolkit::TkData;


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

pub(crate) unsafe fn borrow_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrBorrow;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_borrow(ptr))
    }
}
