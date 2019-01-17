// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use crate::callback::Condition;
use crate::macros::Widget;
use crate::event::{Action, GuiResponse, Handler, NoResponse, err_num, err_unhandled};
use crate::{Class, Coord, Core, CoreData, TkWidget, Widget};

/// A window is a drawable interactive region provided by windowing system.
// TODO: should this be a trait, instead of simply a struct? Should it be
// implemented by dialogs? Note that from the toolkit perspective, it seems a
// Window should be a Widget. So alternatives are (1) use a struct instead of a
// trait or (2) allow any Widget to derive Window (i.e. implement required
// functionality with macros instead of the generic code below).
pub trait Window: Widget {
    /// Upcast
    /// 
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget(&self) -> &Widget;
    /// Upcast, mutably
    /// 
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget_mut(&mut self) -> &mut Widget;
    
    /// Calculate and update positions for all sub-widgets
    #[cfg(feature = "layout")]
    fn configure_widgets(&mut self, tk: &TkWidget);
    
    /// Adjust the size of the window, repositioning widgets.
    /// 
    /// `configure_widgets` must be called before this.
    #[cfg(feature = "layout")]
    fn resize(&mut self, tk: &TkWidget, size: Coord);
    
    /// Handle a high-level event directed at the widget identified by `num`,
    /// and return a user-defined message.
    // NOTE: we could instead add the trait bound Handler<Response = GuiResponse>
    // but (1) Rust doesn't yet support mult-trait objects
    // and (2) https://github.com/rust-lang/rust/issues/57218
    fn handle_action(&mut self, tk: &TkWidget, action: Action, num: u32) -> GuiResponse;
    
    /// Get a list of available callbacks.
    /// 
    /// This returns a sequence of `(index, condition)` values. The toolkit
    /// should call `trigger_callback(index, tk)` whenever the condition is met.
    fn callbacks(&self) -> Vec<(usize, Condition)>;
    
    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &TkWidget);
    
    /// Called by the toolkit after the window has been created and before it
    /// is drawn. This allows callbacks to be invoked "on start".
    fn on_start(&mut self, tk: &TkWidget);
}

/// Main window type
#[layout]
#[widget(class = Class::Window)]
#[derive(Widget)]
pub struct SimpleWindow<W: Widget + 'static> {
    #[core] core: CoreData,
    min_size: Coord,
    #[cfg(feature = "cassowary")] solver: crate::cw::Solver,
    #[widget] w: W,
    fns: Vec<(Condition, &'static Fn(&mut W, &TkWidget))>,
}

impl<W: Widget> Debug for SimpleWindow<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SimpleWindow {{ core: {:?}, min_size: {:?}, solver: <omitted>, w: {:?}, fns: [",
            self.core, self.min_size, self.w)?;
        let mut iter = self.fns.iter();
        if let Some(first) = iter.next() {
            write!(f, "({:?}, <FnMut>)", first.0)?;
            for next in iter {
                write!(f, ", ({:?}, <FnMut>)", next.0)?;
            }
        }
        write!(f, "] }}")
    }
}

impl<W: Widget + Clone> Clone for SimpleWindow<W> {
    fn clone(&self) -> Self {
        SimpleWindow {
            core: self.core.clone(),
            min_size: self.min_size,
            #[cfg(feature = "cassowary")] solver: crate::cw::Solver::new(),
            w: self.w.clone(),
            fns: self.fns.clone(),
        }
    }
}

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow {
            core: Default::default(),
            min_size: (0, 0),
            #[cfg(feature = "cassowary")] solver: crate::cw::Solver::new(),
            w,
            fns: Vec::new(),
        }
    }
    
    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(&mut self, f: &'static Fn(&mut W, &TkWidget),
            conditions: &[Condition])
    {
        for c in conditions {
            self.fns.push((*c, f));
        }
    }
}

impl<R, W: Widget + Handler<Response = R> + 'static> Window
    for SimpleWindow<W>
    where GuiResponse: From<R>, R: From<NoResponse>
{
    fn as_widget(&self) -> &Widget { self }
    fn as_widget_mut(&mut self) -> &mut Widget { self }
    
    #[cfg(feature = "cassowary")]
    fn configure_widgets(&mut self, tk: &TkWidget) {
        use crate::cw;
        assert!(self.number() > 0, "widget not enumerated");
        
        let v_w = cw_var!(self, w);
        let v_h = cw_var!(self, h);
        
        self.solver.reset();
        
        self.w.init_constraints(tk, &mut self.solver, true);
        
        self.solver.add_edit_variable(v_w, cw::strength::MEDIUM * 100.0).unwrap();
        self.solver.add_edit_variable(v_h, cw::strength::MEDIUM * 100.0).unwrap();
        
        self.min_size = (self.solver.get_value(v_w) as i32, self.solver.get_value(v_h) as i32);
        
        self.w.apply_constraints(tk, &self.solver, (0, 0));
    }
    
    #[cfg(feature = "cassowary")]
    fn resize(&mut self, tk: &TkWidget, size: Coord) {
        assert!(self.number() > 0, "widget not enumerated");
        
        self.solver.suggest_value(cw_var!(self, w), size.0 as f64).unwrap();
        self.solver.suggest_value(cw_var!(self, h), size.1 as f64).unwrap();
        
        self.w.apply_constraints(tk, &self.solver, (0, 0));
    }
    
    fn handle_action(&mut self, tk: &TkWidget, action: Action, num: u32) -> GuiResponse {
        if num < self.number() {
            GuiResponse::from(self.w.handle_action(tk, action, num))
        } else if num == self.number() {
            match action {
                Action::Close => GuiResponse::Close,
                _ => err_unhandled(action),
            }
        } else {
            err_num()
        }
    }
    
    fn callbacks(&self) -> Vec<(usize, Condition)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }
    
    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &TkWidget) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, tk);
    }
    
    fn on_start(&mut self, tk: &TkWidget) {
        for cb in &mut self.fns {
            if cb.0 == Condition::Start {
                (cb.1)(&mut self.w, tk);
            }
        }
    }
}
