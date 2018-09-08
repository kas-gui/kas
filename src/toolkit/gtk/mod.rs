//! GTK backend
//! 
//! This will be migrated to a separate library later.

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use gdk;
use gtk;
use gtk::{Cast, WidgetExt, ContainerExt};

use widget::{Class, Widget};
use widget::window::Window;
use toolkit::Toolkit;

unsafe fn extend_lifetime<'b, R: ?Sized>(r: &'b R) -> &'static R {
    ::std::mem::transmute::<&'b R, &'static R>(r)
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
    
    fn add_widgets(&mut self, gtk_widget: gtk::Container, widget: &'static Widget) {
        for child in (0..widget.len()).map(|i| &widget[i]) {
            let gtk_child = match child.class() {
                Class::Container =>
                    gtk::Box::new(gtk::Orientation::Vertical, 3)
                        .upcast::<gtk::Widget>(),
                Class::Button =>
                    gtk::Button::new_with_label(child.label().unwrap())
                        .upcast::<gtk::Widget>(),
                Class::Text => gtk::Label::new(child.label())
                        .upcast::<gtk::Widget>(),
                Class::Window => continue,  // TODO embedded windows?
            };
            gtk_widget.add(&gtk_child);
            if let Ok(gtk_container) = gtk_child.dynamic_cast::<gtk::Container>() {
                self.add_widgets(gtk_container, child);
            }
        }
    }
}

// event handler code
impl GtkToolkit {
    fn remove_window(&self, window: &gtk::Window) {
        let mut windows = self.windows.borrow_mut();
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
        gtk_window.show_all();
        gtk_window.connect_delete_event(|slf, _| {
            TOOLKIT.with(|t| t.get().map(|tk| tk.remove_window(slf)));
            gtk::Inhibit(false)
        });
        let window = Box::new(window);
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        self.add_widgets(gtk_window.clone().upcast::<gtk::Container>(),
            unsafe{ extend_lifetime(&*window) });
        gtk_window.show_all();
        self.windows.get_mut().push((window, gtk_window));
    }
    
    fn main(&mut self) {
        gtk::main();
    }
}


#[derive(Debug)]
pub struct Error(pub &'static str);
