//! `Window` and `WindowList` types

use std::cell::RefCell;

use gtk::{Cast, WidgetExt, ContainerExt, ButtonExt};
#[cfg(not(feature = "layout"))] use gtk::GridExt;

use mygui::event::Action;
use mygui::widget::{Class, Widget};
use mygui::window::{Response};

use crate::widget;
use crate::tkd::WidgetAbstraction;


/// Per-window data
struct Window {
    /// The mygui window. Each is boxed since it must not move.
    pub win: Box<mygui::window::Window>,
    /// The GTK window
    pub gwin: gtk::Window,
    /// Range of widget numbers used, from first to last+1.
    pub nums: (u32, u32),
}

/// A list of windows
/// 
/// This is a special type which has a single instance per thread.
pub(crate) struct WindowList {
    windows: Vec<Window>,
}

// Use thread_local because our type and GTK pointers are not Sync.
thread_local! {
    static WINDOWS: RefCell<WindowList> = RefCell::new(WindowList::new());
}

/// Call some closure on the thread-local window list.
pub(crate) fn with_list<F: FnOnce(&mut WindowList)>(f: F) {
    WINDOWS.with(|cell| f(&mut *cell.borrow_mut()) );
}

impl WindowList {
    const fn new() -> Self {
        WindowList { windows: Vec::new() }
    }
    
    // Find first window with matching `gdk::Window`, run the closure, and
    // return the result, or `None` if no match.
    pub(crate) fn for_gdk_win<T, F>(&mut self, gdk_win: gdk::Window, f: F) -> Option<T>
        where F: FnOnce(&mut mygui::window::Window, &mut gtk::Window) -> T
    {
        let gdk_win = Some(gdk_win);
        for item in self.windows.iter_mut() {
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
        for i in 0..widget.len() {
            let child = widget.get_mut(i).unwrap();
            // TODO: use trait implementation for each different class?
            let gtk_child = match child.class() {
                #[cfg(not(feature = "layout"))]
                Class::Container => {
                    use mygui::widget::ChildLayout;
                    match child.child_layout() {
                        ChildLayout::None |
                        ChildLayout::Horizontal =>
                            gtk::Box::new(gtk::Orientation::Horizontal, 0)
                                .upcast::<gtk::Widget>(),
                        ChildLayout::Vertical =>
                            gtk::Box::new(gtk::Orientation::Vertical, 0)
                                .upcast::<gtk::Widget>(),
                        ChildLayout::Grid =>
                            gtk::Grid::new().upcast::<gtk::Widget>()
                    }
                }
                #[cfg(feature = "layout")]
                Class::Container => {
                    // orientation is unimportant
                    gtk::Box::new(gtk::Orientation::Horizontal, 0)
                                .upcast::<gtk::Widget>()
                }
                Class::Button => {
                    let button = gtk::Button::new_with_label(child.label().unwrap());
                    let num = child.number();
                    button.connect_clicked(move |_| {
                        let action = Action::ButtonClick;
                        with_list(|list| list.handle_action(action, num))
                    });
                    button.upcast::<gtk::Widget>()
                }
                Class::Text => gtk::Label::new(child.label())
                        .upcast::<gtk::Widget>(),
                Class::Window => panic!(),  // TODO embedded windows?
            };
            
            add_widgets(&gtk_child, child);
            
            #[cfg(not(feature = "layout"))] {
                if let Some(grid) = gtk_container.downcast_ref::<gtk::Grid>() {
                    let pos = widget.grid_pos(i).unwrap_or((0, 0, 1, 1));
                    grid.attach(&gtk_child, pos.0, pos.1, pos.2, pos.3);
                    continue;   // attach(...) instead of add(...)
                }
            }
            gtk_container.add(&gtk_child);
        }
    }
}

fn clear_tkd(widget: &mut Widget) {
    (0..widget.len()).for_each(|i| clear_tkd(widget.get_mut(i).unwrap()));
    widget.clear_gw();
}

// event handler code
impl WindowList {
    fn handle_action(&mut self, action: Action, num: u32) {
        let mut remove = None;
        
        for (i, w) in self.windows.iter_mut().enumerate() {
            if num >= w.nums.0 && num < w.nums.1 {
                match w.win.handle_action(&widget::Toolkit, action, num) {
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
            self.windows.remove(i);
            if self.windows.is_empty() {
                gtk::main_quit();
            }
        }
    }
    
    pub(crate) fn add_window(&mut self, mut win: Box<mygui::window::Window>) {
        let gwin = gtk::Window::new(gtk::WindowType::Toplevel);
        
        let num0 = self.windows.last().map(|tw| tw.nums.1).unwrap_or(0);
        let num1 = win.enumerate(num0);
        let num = win.number();
        
        gwin.connect_delete_event(move |_, _| {
            with_list(|list| list.handle_action(Action::Close, num));
            gtk::Inhibit(false)
        });
        
        // HACK: GTK widgets depend on passed pointers but don't mark lifetime
        // restrictions in their types. We cannot guard usage correctly.
        // TODO: we only need lifetime extension if GTK widgets refer to our
        // ones (currently they don't; wait until event handling is implemented)
        add_widgets(gwin.upcast_ref::<gtk::Widget>(), win.as_widget_mut());
        
        gwin.show_all();
        
        self.windows.push(Window {
            win,
            gwin,
            nums: (num0, num1),
        });
    }
}
