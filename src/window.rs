// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use crate::callback::Condition;
use crate::event::{err_num, err_unhandled, Event, Handler, Response};
use crate::macros::Widget;
use crate::{Class, Core, CoreData, Coord, Size, TkWidget, Widget};

/// A window is a drawable interactive region provided by windowing system.
// TODO: should this be a trait, instead of simply a struct? Should it be
// implemented by dialogs? Note that from the toolkit perspective, it seems a
// Window should be a Widget. So alternatives are (1) use a struct instead of a
// trait or (2) allow any Widget to derive Window (i.e. implement required
// functionality with macros instead of the generic code below).
pub trait Window: Widget + Handler<Msg = ()> {
    /// Upcast
    ///
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget(&self) -> &dyn Widget;
    /// Upcast, mutably
    ///
    /// Note: needed because Rust does not yet support trait object upcasting
    fn as_widget_mut(&mut self) -> &mut dyn Widget;

    /// Calculate and update positions for all sub-widgets
    fn configure_widgets(&mut self, tk: &mut dyn TkWidget);

    /// Adjust the size of the window, repositioning widgets.
    ///
    /// `configure_widgets` must be called before this.
    fn resize(&mut self, tk: &mut dyn TkWidget, size: Size);

    /// Get a list of available callbacks.
    ///
    /// This returns a sequence of `(index, condition)` values. The toolkit
    /// should call `trigger_callback(index, tk)` whenever the condition is met.
    fn callbacks(&self) -> Vec<(usize, Condition)>;

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWidget);

    /// Called by the toolkit after the window has been created and before it
    /// is drawn. This allows callbacks to be invoked "on start".
    fn on_start(&mut self, tk: &mut dyn TkWidget);
}

/// The main instantiation of the `Window` trait.
///
/// TODO: change the name?
#[widget(class = Class::Window)]
#[derive(Widget)]
pub struct SimpleWindow<W: Widget + 'static> {
    #[core]
    core: CoreData,
    min_size: Size,
    solver: crate::cw::Solver,
    #[widget]
    w: W,
    fns: Vec<(Condition, &'static dyn Fn(&mut W, &mut dyn TkWidget))>,
}

impl<W: Widget> Debug for SimpleWindow<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SimpleWindow {{ core: {:?}, min_size: {:?}, solver: <omitted>, w: {:?}, fns: [",
            self.core, self.min_size, self.w
        )?;
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
            solver: crate::cw::Solver::new(),
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
            min_size: Size::zero(),
            solver: crate::cw::Solver::new(),
            w,
            fns: Vec::new(),
        }
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(
        &mut self,
        f: &'static dyn Fn(&mut W, &mut dyn TkWidget),
        conditions: &[Condition],
    ) {
        for c in conditions {
            self.fns.push((*c, f));
        }
    }
}

impl<M, W: Widget + Handler<Msg = M> + 'static> Handler for SimpleWindow<W> {
    type Msg = ();

    fn handle(&mut self, tk: &mut dyn TkWidget, event: Event) -> Response<Self::Msg> {
        match event {
            Event::ToChild(num, ev) => {
                if num < self.number() {
                    // TODO: either allow a custom handler or require M=()
                    let r = self.w.handle(tk, Event::ToChild(num, ev));
                    Response::try_from(r).unwrap_or_else(|_| {
                        panic!("TODO: widget returned custom msg to window (currently unsupported)")
                    })
                } else if num == self.number() {
                    match ev {
                        _ => err_unhandled(Event::ToChild(num, ev)),
                    }
                } else {
                    err_num()
                }
            }
            Event::ToCoord(coord, ev) => {
                // widget covers entire area
                let r = self.w.handle(tk, Event::ToCoord(coord, ev));
                Response::try_from(r).unwrap_or_else(|_| {
                    panic!("TODO: widget returned custom msg to window (currently unsupported)")
                })
            }
        }
    }
}

impl<M, W: Widget + Handler<Msg = M> + 'static> Window for SimpleWindow<W> {
    fn as_widget(&self) -> &dyn Widget {
        self
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }

    fn configure_widgets(&mut self, tk: &mut dyn TkWidget) {
        assert!(self.number() > 0, "widget not enumerated");

        let v_w = cw_var!(self, w);
        let v_h = cw_var!(self, h);

        self.solver.reset();

        self.w.init_constraints(tk, &mut self.solver, true);

        self.solver
            .add_edit_variable(v_w, cw::strength::MEDIUM * 100.0)
            .unwrap();
        self.solver
            .add_edit_variable(v_h, cw::strength::MEDIUM * 100.0)
            .unwrap();

        self.min_size = Size(
            self.solver.get_value(v_w) as u32,
            self.solver.get_value(v_h) as u32,
        );

        self.w.apply_constraints(tk, &self.solver, Coord::zero());
    }

    fn resize(&mut self, tk: &mut dyn TkWidget, size: Size) {
        assert!(self.number() > 0, "widget not enumerated");

        self.solver
            .suggest_value(cw_var!(self, w), size.0 as f64)
            .unwrap();
        self.solver
            .suggest_value(cw_var!(self, h), size.1 as f64)
            .unwrap();

        self.w.apply_constraints(tk, &self.solver, Coord::zero());
    }

    fn callbacks(&self) -> Vec<(usize, Condition)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWidget) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, tk);
    }

    fn on_start(&mut self, tk: &mut dyn TkWidget) {
        for cb in &mut self.fns {
            if cb.0 == Condition::Start {
                (cb.1)(&mut self.w, tk);
            }
        }
    }
}
