// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! GTK toolkit for kas

mod event;
mod widget;
mod window;
mod tkd;

use std::marker::PhantomData;
use std::{cell::RefCell, rc::Rc};

pub use glib::BoolError as Error;
use kas::Window;


/// Object used to initialise GTK and create windows.
/// 
/// You should only create a single instance of this type. It is neither
/// `Send` nor `Sync`, thus is constrained to the thread on which it is
/// created. On OS X, it must be created on the "main thread".
#[derive(Clone)]
pub struct Toolkit {
    // we store no real data: it is all thread-local
    _phantom: PhantomData<Rc<()>>,  // not Send or Sync
}

impl Toolkit {
    /// Construct a new instance. This initialises GTK. This should only be
    /// constructed once.
    pub fn new() -> Result<Self, Error> {
        gtk::init()?;
        
        gdk::Event::set_handler(Some(event::handler));
        
        Ok(Toolkit { _phantom: Default::default() })
    }
    
    
    /// Assume ownership of and display a window.
    /// 
    /// Note: typically, one should have `W: Clone`, enabling multiple usage.
    pub fn add<W: Window + 'static>(&self, window: W) where Self: Sized {
        self.add_rc(Rc::new(RefCell::new(window)))
    }
    
    /// Specialised version of `add`; typically toolkits only need to implement
    /// this.
    pub fn add_rc(&self, win: Rc<RefCell<dyn kas::Window>>) {
        window::with_list(|list| list.add_window(win))
    }
    
    /// Run the main loop.
    pub fn main(self) -> () {
        window::with_list(|list| {
            for window in &list.windows {
                window.win.borrow_mut().on_start(&widget::Toolkit);
            }
        });
        gtk::main();
    }
}
