//! GTK backend
//! 
//! This will be migrated to a separate library later.

use std::marker::PhantomData;
use std::rc::Rc;

use gtk;
use gtk::{Cast, WidgetExt, ContainerExt};

use widget::{Class, Widget};
use widget::window::Window;
use toolkit::Toolkit;

unsafe fn extend_lifetime<'b, R: ?Sized>(r: &'b R) -> &'static R {
    ::std::mem::transmute::<&'b R, &'static R>(r)
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
    windows: Vec<(Box<Window>, gtk::Window)>,
    _phantom: PhantomData<Rc<()>>,  // not Send or Sync
}

impl GtkToolkit {
    /// Construct
    pub fn new() -> Result<Self, Error> {
        (gtk::init().map_err(|e| Error(e.0)))?;
        Ok(GtkToolkit {
            windows: Vec::new(),
            _phantom: PhantomData,
        })
    }
    
    /// Run the main loop.
    pub fn main(&mut self) {
        gtk::main();
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

impl Toolkit for GtkToolkit {
    fn add<W: Window+'static>(&mut self, window: W) {
        let gtk_window = gtk::Window::new(gtk::WindowType::Toplevel);
        gtk_window.show_all();
        gtk_window.connect_delete_event(|_, _| {
            gtk::main_quit();
            gtk::Inhibit(false)
        });
        let window = Box::new(window);
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        self.add_widgets(gtk_window.clone().upcast::<gtk::Container>(),
            unsafe{ extend_lifetime(&*window) });
        gtk_window.show_all();
        self.windows.push((window, gtk_window));
    }
}


#[derive(Debug)]
pub struct Error(pub &'static str);
