// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Window widgets

use std::fmt::{self, Debug};

use crate::event::{Address, Callback, Event, Handler, Manager, Response, VoidMsg};
use crate::geom::Size;
use crate::layout::{self};
use crate::macros::Widget;
use crate::theme::SizeHandle;
use crate::{CoreData, LayoutData, Widget};

/// The main instantiation of the [`Window`] trait.
#[widget]
#[layout(single)]
#[derive(Widget)]
pub struct Window<W: Widget + 'static> {
    #[core]
    core: CoreData,
    #[layout_data]
    layout_data: <Self as LayoutData>::Data,
    enforce_min: bool,
    enforce_max: bool,
    title: String,
    #[widget]
    w: W,
    fns: Vec<(Callback, &'static dyn Fn(&mut W, &mut Manager))>,
    final_callback: Option<&'static dyn Fn(Box<dyn kas::Window>, &mut Manager)>,
}

impl<W: Widget> Debug for Window<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Window {{ core: {:?}, solver: <omitted>, w: {:?}, fns: [",
            self.core, self.w
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
            enforce_min: self.enforce_min,
            enforce_max: self.enforce_max,
            title: self.title.clone(),
            w: self.w.clone(),
            fns: self.fns.clone(),
            final_callback: self.final_callback.clone(),
        }
    }
}

impl<W: Widget> Window<W> {
    /// Create
    pub fn new<T: ToString>(title: T, w: W) -> Window<W> {
        Window {
            core: Default::default(),
            layout_data: Default::default(),
            enforce_min: true,
            enforce_max: false,
            title: title.to_string(),
            w,
            fns: Vec::new(),
            final_callback: None,
        }
    }

    /// Configure whether min/max dimensions are forced
    ///
    /// By default, the min size is enforced but not the max.
    pub fn set_enforce_size(&mut self, min: bool, max: bool) {
        self.enforce_min = min;
        self.enforce_max = max;
    }

    /// Add a closure to be called, with a reference to self, on the given
    /// condition. The closure must be passed by reference.
    pub fn add_callback(&mut self, condition: Callback, f: &'static dyn Fn(&mut W, &mut Manager)) {
        self.fns.push((condition, f));
    }

    /// Set a callback to be called when the window is closed.
    ///
    /// This callback assumes ownership of self, with the advantages and
    /// disadvantages (type erasure) that this implies. Alternatively, one can
    /// use [`Window::add_callback`] with [`Callback::Close`].
    ///
    /// Only a single callback is allowed; if another exists it is replaced.
    pub fn set_final_callback(&mut self, f: &'static dyn Fn(Box<dyn kas::Window>, &mut Manager)) {
        self.final_callback = Some(f);
    }
}

impl<W: Widget + Handler<Msg = VoidMsg> + 'static> Handler for Window<W> {
    type Msg = VoidMsg;

    fn handle(&mut self, mgr: &mut Manager, addr: Address, event: Event) -> Response<Self::Msg> {
        // The window itself doesn't handle events, so we can just pass through
        self.w.handle(mgr, addr, event)
    }
}

impl<W: Widget + Handler<Msg = VoidMsg> + 'static> kas::Window for Window<W> {
    fn title(&self) -> &str {
        &self.title
    }

    fn resize(
        &mut self,
        size_handle: &mut dyn SizeHandle,
        size: Size,
    ) -> (Option<Size>, Option<Size>) {
        let (min, max) = layout::solve(self, size_handle, size);
        (
            if self.enforce_min { Some(min) } else { None },
            if self.enforce_max { Some(max) } else { None },
        )
    }

    fn callbacks(&self) -> Vec<(usize, Callback)> {
        self.fns.iter().map(|(cond, _)| *cond).enumerate().collect()
    }

    fn final_callback(&self) -> Option<&'static dyn Fn(Box<dyn kas::Window>, &mut Manager)> {
        self.final_callback
    }

    /// Trigger a callback (see `iter_callbacks`).
    fn trigger_callback(&mut self, index: usize, mgr: &mut Manager) {
        let cb = &mut self.fns[index].1;
        cb(&mut self.w, mgr);
    }
}
