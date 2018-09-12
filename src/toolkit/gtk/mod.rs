//! GTK backend
//! 
//! This will be migrated to a separate library later.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::{align_of, size_of, transmute};
use std::ops::Deref;
use std::rc::Rc;

use gdk;
use gtk;
use gtk::{Cast, WidgetExt, ContainerExt};

use {Coord, Rect};
use widget::{Class, Widget};
use widget::window::Window;
use toolkit::{Toolkit, TkData, TkWidget};

unsafe fn extend_lifetime<'b, R: ?Sized>(r: &'b R) -> &'static R {
    ::std::mem::transmute::<&'b R, &'static R>(r)
}

unsafe fn extend_lifetime_mut<'b, R: ?Sized>(r: &'b mut R) -> &'static mut R {
    ::std::mem::transmute::<&'b mut R, &'static mut R>(r)
}

fn handler(event: &mut gdk::Event) {
    use gdk::EventType::*;
    match event.get_event_type() {
        Nothing => return,  // ignore this event
        
        // let GTK handle these for now:
        ButtonPress |
        ButtonRelease |
        ClientEvent |
        Configure |     // TODO: layout
        Damage |
        Delete |
        DoubleButtonPress |
        EnterNotify |
        Expose |
        FocusChange |
        GrabBroken |
        KeyPress |
        KeyRelease |
        LeaveNotify |
        Map |
        MotionNotify |
        PropertyNotify |
        SelectionClear |
        SelectionNotify |
        SelectionRequest |
        Setting |
        TripleButtonPress |
        Unmap |
        VisibilityNotify |
        WindowState => {
            // fall through
        },
        
        _ => {
            println!("Event: {:?}", event);
        }
    }
    gtk::main_do_event(event);
}

/// Object used to initialise GTK and create windows
/// 
/// This toolkit is neither `Send` nor `Sync`. Additionally, on OS X, it must
/// live in the "main thread". On all platforms, only a single instance should
/// exist at any time (TODO: or what?).
/// 
/// TODO: do we want this to assume ownership of things and heap-allocate?
/// Perhaps: yes. But since we must copy, only take a `&Window` in `show`.
/// Then: re-build all widgets with desired associated data. Allocate the whole
/// structure on the heap so that values never get moved. Now set references.
pub struct GtkToolkit {
    // Note: `Box<_>` values must exist for as long as last param
    windows: RefCell<Vec<(Box<Window>, gtk::Window)>>,
    _phantom: PhantomData<Rc<()>>,  // not Send or Sync
}

// Use thread_local because our type and GTK pointers are not Sync.
thread_local! {
    static TOOLKIT: Cell<Option<&'static GtkToolkit>> = Cell::new(None);
}

impl GtkToolkit {
    /// Construct
    pub fn new() -> Result<Box<Self>, Error> {
        if TOOLKIT.with(|t| t.get().is_some()) {
            return Err(Error("GtkToolkit::new(): can only be called once"));
        }
        
        (gtk::init().map_err(|e| Error(e.0)))?;
        
        unsafe{ gdk::Event::set_handler(Some(handler)); }
        
        let tk = Box::new(GtkToolkit {
            windows: RefCell::new(Vec::new()),
            _phantom: PhantomData,
        });
        
        // Cannot use static lifetime analysis here, so we rely on Drop to clean up
        let p = Some(unsafe { extend_lifetime(tk.deref()) });
        TOOLKIT.with(|t| t.set(p));
        Ok(tk)
    }
    
    fn add_widgets(&mut self, gtk_widget: &gtk::Widget, widget: &mut Widget) {
        widget.set_tkd(unsafe { own_to_tkd(gtk_widget) });
        if let Some(gtk_container) = gtk_widget.downcast_ref::<gtk::Container>() {
            (0..widget.len()).for_each(|i| {
                let child = widget.get_mut(i).unwrap();
                let gtk_child = match child.class() {
                    Class::Container =>
                        gtk::Box::new(gtk::Orientation::Vertical, 3)
                            .upcast::<gtk::Widget>(),
                    Class::Button =>
                        gtk::Button::new_with_label(child.label().unwrap())
                            .upcast::<gtk::Widget>(),
                    Class::Text => gtk::Label::new(child.label())
                            .upcast::<gtk::Widget>(),
                    Class::Window => panic!(),  // TODO embedded windows?
                };
                gtk_container.add(&gtk_child);
                self.add_widgets(&gtk_child, child);
            });
        }
    }
    
    fn clear_tkd<'a, 'b>(&'a self, widget: &'b mut Widget) {
        (0..widget.len()).for_each(|i| self.clear_tkd(widget.get_mut(i).unwrap()));
        
        // convert back to smart pointer to reduce reference count
        if let Some(_) = unsafe { own_from_tkd(widget.get_tkd()) } {
            // mark empty
            widget.set_tkd(Default::default());
        }
    }
}

// event handler code
impl GtkToolkit {
    fn remove_window(&self, window: &gtk::Window) {
        let mut windows = self.windows.borrow_mut();
        for w in windows.iter_mut() {
            if w.1 == *window {
                self.clear_tkd(w.0.as_widget_mut());
            }
        }
        windows.retain(|w| {
            w.1 != *window
        });
        if windows.is_empty() {
            gtk::main_quit();
        }
    }
}

impl Drop for GtkToolkit {
    fn drop(&mut self) {
        TOOLKIT.with(|t| t.set(None));
    }
}

impl Toolkit for GtkToolkit {
    fn add<W: Window+'static>(&mut self, window: W) {
        let gtk_window = gtk::Window::new(gtk::WindowType::Toplevel);
        gtk_window.connect_delete_event(|slf, _| {
            TOOLKIT.with(|t| t.get().map(|tk| tk.remove_window(slf)));
            gtk::Inhibit(false)
        });
        
        let mut window = Box::new(window);
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        // TODO: we only need lifetime extension if GTK widgets refer to our
        // ones (currently they don't; wait until event handling is implemented)
        self.add_widgets(gtk_window.upcast_ref::<gtk::Widget>(),
            unsafe{ extend_lifetime_mut(&mut *window) });
        
        window.configure_widgets(self);
        gtk_window.show_all();
        
        self.windows.get_mut().push((window, gtk_window));
    }
    
    fn main(&mut self) {
        gtk::main();
    }
    
    fn tk_widget(&self) -> &TkWidget {
        self
    }
}

unsafe fn own_to_tkd(w: &gtk::Widget) -> TkData {
    use glib::translate::ToGlibPtr;
    let ptr = gtk::Widget::to_glib_full(w);
    let mut tkd = TkData::default();
    tkd.0 = transmute::<*mut ::gtk_sys::GtkWidget, u64>(ptr);
    tkd
}

unsafe fn own_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrFull;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_full(ptr))
    }
}

unsafe fn borrow_from_tkd(tkd: TkData) -> Option<gtk::Widget> {
    use glib::translate::FromGlibPtrBorrow;
    if tkd.0 == 0 {
        None
    } else {
        let ptr = transmute::<u64, *mut ::gtk_sys::GtkWidget>(tkd.0);
        Some(gtk::Widget::from_glib_borrow(ptr))
    }
}

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


#[derive(Debug)]
pub struct Error(pub &'static str);
