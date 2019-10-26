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
    size_pref: SizePref,
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
            size_pref: self.size_pref,
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
        which: usize,
    ) -> Size {
        self.w.size_pref(tk, pref, axes, which)
    }

    fn set_rect(&mut self, rect: Rect) {
        self.core_data_mut().rect = rect;
        self.w.set_rect(rect);
    }
}

impl<W: Widget> SimpleWindow<W> {
    /// Create
    pub fn new(w: W) -> SimpleWindow<W> {
        SimpleWindow {
            core: Default::default(),
            min_size: Size::ZERO,
            w,
            size_pref: SizePref::Default,
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

        let mut pref = self.size_pref;
        let mut axes = Axes::Both;
        let mut s = self.size_pref(tk, pref, axes, 0);

        // Last two valid sizes
        let mut b = [s, s];
        let mut which = true;
        let mut which_vert = which;

        let init_dir0 = Dir::from(s.0, size.0);
        let init_dir1 = Dir::from(s.1, size.1);
        let (mut dir0, mut dir1) = (init_dir0, init_dir1);
        while dir0 == init_dir0 {
            if dir1 != dir0 {
                axes = Axes::Horiz;
            }
            if dir0.adjust(&mut pref) {
                break;
            }
            s = self.size_pref(tk, pref, axes, which as usize);
            b[which as usize].0 = s.0;
            dir0 = Dir::from(s.0, size.0);
            if axes == Axes::Both {
                b[which as usize].1 = s.1;
                dir1 = Dir::from(s.1, size.1);
                which_vert = !which;
            }
            which = !which;
        }

        // Remember final value from first loop only
        self.size_pref = pref;

        axes = Axes::Vert;
        while dir1 == init_dir1 {
            if dir1.adjust(&mut pref) {
                break;
            }
            s = self.size_pref(tk, pref, axes, which_vert as usize);
            b[which_vert as usize].1 = s.1;
            dir1 = Dir::from(s.1, size.1);
            which_vert = !which_vert;
        }

        // Using sizes outside this range is invalid
        let min_s = Size(b[0].0.min(b[1].0), b[0].1.min(b[1].1));
        let max_s = Size(b[0].0.max(b[1].0), b[0].1.max(b[1].1));
        s.0 = size.0.max(min_s.0).min(max_s.0);
        s.1 = size.1.max(min_s.1).min(max_s.1);

        let rect = Rect {
            pos: Coord::ZERO,
            size: s,
        };
        self.set_rect(rect);

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
