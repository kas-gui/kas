//! GTK backend
//! 
//! This will be migrated to a separate library later.

use std::marker::PhantomData;
use std::rc::Rc;

use gtk;
use gtk::WidgetExt;

use widget::window::Window;
use toolkit::Toolkit;


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
}

impl Toolkit for GtkToolkit {
    fn add<W: Window+'static>(&mut self, window: W) {
        let w = gtk::Window::new(gtk::WindowType::Toplevel);
        w.show_all();
        w.connect_delete_event(|_, _| {
            gtk::main_quit();
            gtk::Inhibit(false)
        });
        self.windows.push((Box::new(window), w));
    }
}


#[derive(Debug)]
pub struct Error(pub &'static str);
