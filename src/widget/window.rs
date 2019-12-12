// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use crate::event::{Callback, Event, Handler, Response, VoidMsg};
use crate::geom::Size;
use crate::layout::{self};
use crate::macros::Widget;
use crate::{CoreData, LayoutData, TkWindow, Widget};

/// The main instantiation of the [`Window`] trait.
#[widget(layout = single)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as LayoutData>::Data,
    min_size: Size,
    #[widget]
    w: W,
    fns: Vec<(Callback, &'static dyn Fn(&mut W, &mut dyn TkWindow))>,
}

impl<W: Widget> Debug for Window<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Window {{ core: {:?}, min_size: {:?}, solver: <omitted>, w: {:?}, fns: [",
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

impl<W: Widget + Clone> Clone for Window<W> {
    fn clone(&self) -> Self {
        Window {
            core: self.core.clone(),
            layout_data: self.layout_data.clone(),
            min_size: self.min_size,
            w: self.w.clone(),
            fns: self.fns.clone(),
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new(w: W) -> Window<W> {
        Window {
            core: Default::default(),
            layout_data: Default::default(),
            min_size: Size::ZERO,
            w,
            fns: Vec::new(),
        }
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(
        &mut self,
        condition: Callback,
        f: &'static dyn Fn(&mut W, &mut dyn TkWindow),
    ) {
        self.fns.push((condition, f));
    }
}

impl<W: Widget + Handler<Msg = VoidMsg> + 'static> Handler for Window<W> {
    type Msg = VoidMsg;

    fn handle(&mut self, tk: &mut dyn TkWindow, event: Event) -> Response<Self::Msg> {
        // The window itself doesn't handle events, so we can just pass through
        self.w.handle(tk, event)
    }
}

impl<W: Widget + Handler<Msg = VoidMsg> + 'static> kas::Window for Window<W> {
    fn resize(&mut self, tk: &mut dyn TkWindow, size: Size) {
        layout::solve(self, tk, size);
    }

    fn callbacks(&self) -> Vec<(usize, Callback)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, tk: &mut dyn TkWindow) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, tk);
    }
}
