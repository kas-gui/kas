// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widgets

use std::iter;

use crate::event::{Address, Event, Handler, Manager, Response};
use crate::layout::{self, AxisInfo, Direction, Margins, RulesSetter, RulesSolver, SizeRules};
use crate::theme::{DrawHandle, SizeHandle};
use crate::{CoreData, TkAction, TkWindow, Widget, WidgetCore};
use kas::geom::Rect;

/// A dynamic row/column widget
///
/// This is simply a parameterisation of [`DynVec`] which uses `Box<dyn Widget>`
/// as the parameter type.
pub type DynList<D> = DynVec<D, Box<dyn Widget>>;

/// A dynamic row/column widget
///
/// Generic over a single `Sized` child widget type.
#[derive(Clone, Default, Debug)]
pub struct DynVec<D: Direction, W: Widget> {
    core: CoreData,
    widgets: Vec<W>,
    data: layout::DynRowStorage,
    direction: D,
}

// We implement this manually, because the derive implementation cannot handle
// vectors of child widgets.
impl<D: Direction, W: Widget> WidgetCore for DynVec<D, W> {
    fn core_data(&self) -> &CoreData {
        &self.core
    }
    fn core_data_mut(&mut self) -> &mut CoreData {
        &mut self.core
    }

    fn as_widget(&self) -> &dyn Widget {
        self
    }
    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }

    fn len(&self) -> usize {
        self.widgets.len()
    }
    fn get(&self, index: usize) -> Option<&dyn Widget> {
        self.widgets.get(index).map(|w| w.as_widget())
    }
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Widget> {
        self.widgets.get_mut(index).map(|w| w.as_widget_mut())
    }

    fn walk(&self, f: &mut dyn FnMut(&dyn Widget)) {
        for child in &self.widgets {
            child.walk(f);
        }
        f(self)
    }
    fn walk_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        for child in &mut self.widgets {
            child.walk_mut(f);
        }
        f(self)
    }
}

impl<D: Direction, W: Widget> Widget for DynVec<D, W> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let mut solver = layout::RowSolver::<Vec<u32>, _>::new(
            axis,
            (self.direction, self.widgets.len()),
            &mut self.data,
        );
        for (n, child) in self.widgets.iter_mut().enumerate() {
            solver.for_child(&mut self.data, n, |axis| {
                child.size_rules(size_handle, axis)
            });
        }
        solver.finish(&mut self.data, iter::empty(), iter::empty())
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect) {
        self.core.rect = rect;
        let mut setter = layout::RowSetter::<D, Vec<u32>, _>::new(
            rect,
            Margins::ZERO,
            (self.direction, self.widgets.len()),
            &mut self.data,
        );

        for (n, child) in self.widgets.iter_mut().enumerate() {
            child.set_rect(size_handle, setter.child_rect(n));
        }
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, ev_mgr: &Manager) {
        for child in &self.widgets {
            child.draw(draw_handle, ev_mgr);
        }
    }
}

impl<D: Direction, W: Widget + Handler> Handler for DynVec<D, W> {
    type Msg = <W as Handler>::Msg;

    fn handle(
        &mut self,
        tk: &mut dyn TkWindow,
        addr: Address,
        event: Event,
    ) -> Response<Self::Msg> {
        match addr {
            kas::event::Address::Id(id) => {
                for child in &mut self.widgets {
                    if id <= child.id() {
                        return child.handle(tk, addr, event);
                    }
                }
                debug_assert!(id == self.id(), "Handler::handle: bad WidgetId");
            }
            kas::event::Address::Coord(coord) => {
                for child in &mut self.widgets {
                    if child.rect().contains(coord) {
                        return child.handle(tk, addr, event);
                    }
                }
            }
        }
        Response::Unhandled(event)
    }
}

impl<D: Direction, W: Widget> DynVec<D, W> {
    /// Construct a new instance
    pub fn new(direction: D, widgets: Vec<W>) -> Self {
        DynVec {
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction,
        }
    }

    /// Add a child widget
    pub fn push(&mut self, tk: &mut dyn TkWindow, child: W) {
        self.widgets.push(child);
        tk.send_action(TkAction::Reconfigure);
    }

    /// Resize, using the given closure to construct new widgets
    pub fn resize_with<F: Fn(usize) -> W>(&mut self, tk: &mut dyn TkWindow, len: usize, f: F) {
        let l0 = self.widgets.len();
        if l0 == len {
            return;
        } else if l0 > len {
            self.widgets.truncate(len);
        } else {
            self.widgets.reserve(len);
            for i in l0..len {
                self.widgets.push(f(i));
            }
        }
        tk.send_action(TkAction::Reconfigure);
    }
}
