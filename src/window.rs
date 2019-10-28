// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use crate::callback::Condition;
use crate::event::{err_num, err_unhandled, Event, Handler, Response};
use crate::macros::Widget;
use crate::{Axes, Class, Coord, Core, CoreData, Layout, Rect, Size, SizePref, TkWidget, Widget};

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

    /// Adjust the size of the window, repositioning widgets.
    fn resize(&mut self, tk: &mut dyn TkWidget, size: Size);

    /// Get a list of available callbacks.
    ///
    /// This returns a sequence of `(index, condition)` values. The toolkit
    /// should call `trigger_callback(index, tk)` whenever the condition is met.
    fn callbacks(&self) -> Vec<(usize, Condition)>;

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWidget);
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
            write!(f, "({:?}, <Fn>)", first.0)?;
            for next in iter {
                write!(f, ", ({:?}, <Fn>)", next.0)?;
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
            w: self.w.clone(),
            fns: self.fns.clone(),
        }
    }
}

impl<W: Widget> Layout for SimpleWindow<W> {
    fn size_pref(
        &mut self,
        tk: &mut dyn TkWidget,
        pref: SizePref,
        axes: Axes,
        index: bool,
    ) -> Size {
        self.w.size_pref(tk, pref, axes, index)
    }

    fn set_rect(&mut self, rect: Rect, axes: Axes) {
        self.core_data_mut().rect = rect;
        self.w.set_rect(rect, axes);
    }
}

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow {
            core: Default::default(),
            min_size: Size::ZERO,
            w,
            fns: Vec::new(),
        }
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(
        &mut self,
        condition: Condition,
        f: &'static dyn Fn(&mut W, &mut dyn TkWidget),
    ) {
        self.fns.push((condition, f));
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
                        println!("TODO: widget returned custom msg to window");
                        Response::None
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
                    println!("TODO: widget returned custom msg to window");
                    Response::None
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

    fn resize(&mut self, tk: &mut dyn TkWidget, size: Size) {
        #[derive(Copy, Clone, PartialEq)]
        enum Dir {
            Incr,
            Decr,
            Stop,
        };
        impl Dir {
            fn from(x: u32, y: u32) -> Dir {
                if x < y {
                    Dir::Incr
                } else if x > y {
                    Dir::Decr
                } else {
                    Dir::Stop
                }
            }
            fn adjust(&self, pref: &mut SizePref) -> bool {
                let new = match self {
                    Dir::Incr => pref.increment(),
                    Dir::Decr => pref.decrement(),
                    Dir::Stop => *pref,
                };
                if new == *pref {
                    true
                } else {
                    *pref = new;
                    false
                }
            }
        }

        // Note: it might be tempting to cache pref, but this can lead to unstable behaviour.
        let mut pref = SizePref::Default;
        let mut axes = Axes::Both;
        let mut which = false;
        let mut s = self.size_pref(tk, pref, axes, which);
        which = !which;

        // Last two valid sizes
        let mut b = [s, s];

        let mut dir0 = Dir::from(s.0, size.0);
        let mut dir1 = Dir::from(s.1, size.1);
        let init_dir = dir0;

        // Stage 1a: adjust both dimensions in step
        while dir0 == dir1 && dir0 == init_dir {
            if dir0.adjust(&mut pref) {
                break;
            }
            s = self.size_pref(tk, pref, axes, which);
            b[which as usize] = s;
            dir0 = Dir::from(s.0, size.0);
            dir1 = Dir::from(s.1, size.1);
            which = !which;
        }

        // Stage 1b: continue adjusting horizontally; ignore verticals
        // TODO: generalise, prioritising most restricted dimension?
        axes = Axes::Horiz(false);
        let which1 = which;

        while dir0 == init_dir {
            if dir0.adjust(&mut pref) {
                break;
            }
            s = self.size_pref(tk, pref, axes, which);
            b[which as usize].0 = s.0;
            dir0 = Dir::from(s.0, size.0);
            which = !which;
        }

        // Stage 2a: calculate final horizontal size and fix
        let range = (b[0].0.min(b[1].0), b[0].0.max(b[1].0));
        let dim = size.0.max(range.0).min(range.1);
        let dim2 = b[0].1; // any valid value
        let pos = Coord::ZERO;
        s = Size(dim, dim2);
        self.set_rect(Rect { pos, size: s }, axes);

        // Stage 2b: calculate vertical axis with fixed horizontal
        axes = Axes::Vert(true);
        which = which1;
        dir1 = Dir::from(s.1, size.1);
        let init_dir = dir1;

        while dir1 == init_dir {
            if dir1.adjust(&mut pref) {
                break;
            }
            s = self.size_pref(tk, pref, axes, which);
            b[which as usize].1 = s.1;
            dir1 = Dir::from(s.1, size.1);
            which = !which;
        }

        // Using sizes outside this range is invalid
        let range = (b[0].1.min(b[1].1), b[0].1.max(b[1].1));
        let dim2 = size.1.max(range.0).min(range.1);

        s = Size(dim, dim2);
        self.set_rect(Rect { pos, size: s }, axes);

        // println!("SimpleWindow:");
        // self.w.print_hierarchy(0);
    }

    fn callbacks(&self) -> Vec<(usize, Condition)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWidget) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, tk);
    }
}
