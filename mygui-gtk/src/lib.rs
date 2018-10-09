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

use gtk::{Cast, WidgetExt, ContainerExt, ButtonExt};

use mygui::event::Action;
use mygui::widget::{Class, Widget};
use mygui::widget::window::{Window, Response};
use mygui::toolkit::{Toolkit, TkWidget};

use self::tkd::WidgetAbstraction;

unsafe fn extend_lifetime<'b, R: ?Sized>(r: &'b R) -> &'static R {
    ::std::mem::transmute::<&'b R, &'static R>(r)
}

unsafe fn extend_lifetime_mut<'b, R: ?Sized>(r: &'b mut R) -> &'static mut R {
    ::std::mem::transmute::<&'b mut R, &'static mut R>(r)
}

struct TkWindow {
    /// The mygui window
    pub win: Box<Window>,
    /// The GTK window
    pub gwin: gtk::Window,
    /// Last widget number used + 1. This is the first number available to the
    /// next window.
    pub nend: u32,
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
    windows: RefCell<Vec<TkWindow>>,
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
            if item.gwin.get_window() == gdk_win {
                return Some(f(&mut *item.win, &mut item.gwin))
            }
        }
        None
    }
}

fn add_widgets(gtk_widget: &gtk::Widget, widget: &mut Widget) {
    widget.set_gw(gtk_widget);
    if let Some(gtk_container) = gtk_widget.downcast_ref::<gtk::Container>() {
        (0..widget.len()).for_each(|i| {
            let child = widget.get_mut(i).unwrap();
            // TODO: use trait implementation for each different class?
            let gtk_child = match child.class() {
                #[cfg(not(feature = "layout"))]
                Class::Container => {
                    use mygui::widget::ChildLayout;
                    match child.child_layout() {
                        ChildLayout::None |
                        ChildLayout::Horizontal =>
                            gtk::Box::new(gtk::Orientation::Horizontal, 3)
                                .upcast::<gtk::Widget>(),
                        ChildLayout::Vertical =>
                            gtk::Box::new(gtk::Orientation::Vertical, 3)
                                .upcast::<gtk::Widget>(),
                        ChildLayout::Grid =>
                            // TODO: need to use grid_attach for children!
                            gtk::Grid::new().upcast::<gtk::Widget>()
                    }
                }
                #[cfg(feature = "layout")]
                Class::Container => {
                    // orientation is unimportant
                    gtk::Box::new(gtk::Orientation::Horizontal, 3)
                                .upcast::<gtk::Widget>()
                }
                Class::Button => {
                    let button = gtk::Button::new_with_label(child.label().unwrap());
                    let num = child.get_number();
                    button.connect_clicked(move |_| {
                        let action = Action::ButtonClick;
                        for_toolkit(|tk| tk.handle_action(action, num))
                    });
                    button.upcast::<gtk::Widget>()
                }
                Class::Text => gtk::Label::new(child.label())
                        .upcast::<gtk::Widget>(),
                Class::Window => panic!(),  // TODO embedded windows?
            };
            gtk_container.add(&gtk_child);
            add_widgets(&gtk_child, child);
        });
    }
}

fn clear_tkd(widget: &mut Widget) {
    (0..widget.len()).for_each(|i| clear_tkd(widget.get_mut(i).unwrap()));
    widget.clear_gw();
}

// event handler code
impl GtkToolkit {
    fn handle_action(&self, action: Action, num: u32) {
        let mut windows = self.windows.borrow_mut();
        let mut remove = None;
        
        for (i, w) in windows.iter_mut().enumerate() {
            if num < w.nend {
                match w.win.handle_action(self, action, num) {
                    Response::None => (),
                    Response::Close => {
                        clear_tkd(w.win.as_widget_mut());
                        remove = Some(i);
                    }
                }
                break;
            }
        }
        
        if let Some(i) = remove {
            windows.remove(i);
            if windows.is_empty() {
                gtk::main_quit();
            }
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
        let gwin = gtk::Window::new(gtk::WindowType::Toplevel);
        
        let mut win = Box::new(window.clone());
        let n = self.windows.get_mut().last().map(|tw| tw.nend).unwrap_or(0);
        let nend = win.enumerate(n);
        let num = win.get_number();
        
        gwin.connect_delete_event(move |_, _| {
            for_toolkit(|tk| tk.handle_action(Action::Close, num));
            gtk::Inhibit(false)
        });
        
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        // TODO: we only need lifetime extension if GTK widgets refer to our
        // ones (currently they don't; wait until event handling is implemented)
        add_widgets(gwin.upcast_ref::<gtk::Widget>(),
            unsafe{ extend_lifetime_mut(&mut *win) });
        
        gwin.show_all();
        
        self.windows.get_mut().push(TkWindow {
            win,
            gwin,
            nend
        });
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
