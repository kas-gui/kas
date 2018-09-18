//! GTK toolkit for mygui

extern crate mygui;
extern crate glib;
extern crate gdk;
extern crate gtk;
extern crate gtk_sys;

mod event;
mod widget;
mod tkd;

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use gtk::{Cast, WidgetExt, ContainerExt};

use mygui::widget::{Class, Widget};
use mygui::widget::window::Window;
use mygui::toolkit::{Toolkit, TkWidget};

use self::tkd::{own_to_tkd, own_from_tkd};

unsafe fn extend_lifetime<'b, R: ?Sized>(r: &'b R) -> &'static R {
    ::std::mem::transmute::<&'b R, &'static R>(r)
}

unsafe fn extend_lifetime_mut<'b, R: ?Sized>(r: &'b mut R) -> &'static mut R {
    ::std::mem::transmute::<&'b mut R, &'static mut R>(r)
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

/// Call some closure on the toolkit singleton, if it exists.
fn for_toolkit<F: FnOnce(&GtkToolkit)>(f: F) {
    TOOLKIT.with(|t| t.get().map(f));
}

impl GtkToolkit {
    /// Construct
    pub fn new() -> Result<Box<Self>, Error> {
        if TOOLKIT.with(|t| t.get().is_some()) {
            return Err(Error("GtkToolkit::new(): can only be called once"));
        }
        
        (gtk::init().map_err(|e| Error(e.0)))?;
        
        gdk::Event::set_handler(Some(event::handler));
        
        let tk = Box::new(GtkToolkit {
            windows: RefCell::new(Vec::new()),
            _phantom: PhantomData,
        });
        
        // Cannot use static lifetime analysis here, so we rely on Drop to clean up
        let p = Some(unsafe { extend_lifetime(tk.deref()) });
        TOOLKIT.with(|t| t.set(p));
        Ok(tk)
    }
    
    // Find first window with matching `gdk::Window`, run the closure, and
    // return the result, or `None` if no match.
    fn for_gdk_win<T, F>(&self, gdk_win: gdk::Window, f: F) -> Option<T>
        where F: FnOnce(&mut Window, &mut gtk::Window) -> T
    {
        let mut windows = self.windows.borrow_mut();
        let gdk_win = Some(gdk_win);
        for item in windows.iter_mut() {
            if item.1.get_window() == gdk_win {
                return Some(f(&mut *item.0, &mut item.1))
            }
        }
        None
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
    fn add<W: Clone+Window+'static>(&mut self, window: &W) {
        let gtk_window = gtk::Window::new(gtk::WindowType::Toplevel);
        gtk_window.connect_delete_event(|slf, _| {
            for_toolkit(|tk| tk.remove_window(slf));
            gtk::Inhibit(false)
        });
        
        let mut window = Box::new(window.clone());
        window.enumerate(0);
        
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        // TODO: we only need lifetime extension if GTK widgets refer to our
        // ones (currently they don't; wait until event handling is implemented)
        self.add_widgets(gtk_window.upcast_ref::<gtk::Widget>(),
            unsafe{ extend_lifetime_mut(&mut *window) });
        
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


#[derive(Debug)]
pub struct Error(pub &'static str);
