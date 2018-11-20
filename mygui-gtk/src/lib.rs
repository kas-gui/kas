//! GTK toolkit for mygui

#![feature(const_vec_new)]

mod event;
mod widget;
mod window;
mod tkd;

use std::marker::PhantomData;
use std::rc::Rc;


/// Object used to initialise GTK and create windows.
/// 
/// You should only create a single instance of this type. It is neither
/// `Send` nor `Sync`, thus is constrained to the thread on which it is
/// created. On OS X, it must be created on the "main thread".
pub struct Toolkit {
    // we store no real data: it is all thread-local
    _phantom: PhantomData<Rc<()>>,  // not Send or Sync
}

impl Toolkit {
    /// Construct a new instance. This initialises GTK. This should only be
    /// constructed once.
    pub fn new() -> Result<Self, Error> {
        (gtk::init().map_err(|e| Error(e.0)))?;
        
        gdk::Event::set_handler(Some(event::handler));
        
        Ok(Toolkit { _phantom: Default::default() })
    }
}

impl mygui::toolkit::Toolkit for Toolkit {
    fn add_boxed(&self, win: Box<mygui::window::Window>) {
        window::with_list(|list| list.add_window(win))
    }
    
    fn main(&mut self) {
        gtk::main();
    }
    
    fn tk_widget(&self) -> &mygui::toolkit::TkWidget {
        &widget::Toolkit
    }
}


/// Error type.
#[derive(Debug)]
pub struct Error(pub &'static str);
