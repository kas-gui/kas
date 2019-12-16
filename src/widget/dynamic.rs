// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Dynamic widgets

use std::iter;

use crate::event::Manager;
use crate::layout::{self, AxisInfo, Direction, Margins, RulesSetter, RulesSolver, SizeRules};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle};
use crate::{CoreData, TkAction, TkWindow, Widget};
use kas::geom::Rect;

/// A dynamic row/column widget
#[widget]
#[handler]
#[derive(Default, Debug, Widget)]
pub struct DynList<D: Direction> {
    #[core]
    core: CoreData,
    widgets: Vec<Box<dyn Widget>>,
    data: layout::DynRowStorage,
    direction: D,
}

impl<D: Direction> Clone for DynList<D> {
    fn clone(&self) -> Self {
        DynList {
            core: self.core.clone(),
            widgets: vec![],
            data: self.data.clone(),
            direction: self.direction,
        }
    }
}

impl<D: Direction> Widget for DynList<D> {
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

impl<D: Direction> DynList<D> {
    /// Construct a new instance
    pub fn new(direction: D, widgets: Vec<Box<dyn Widget>>) -> Self {
        DynList {
            core: Default::default(),
            widgets,
            data: Default::default(),
            direction,
        }
    }

    /// Add a child widget
    pub fn push(&mut self, tk: &mut dyn TkWindow, child: Box<dyn Widget>) {
        self.widgets.push(child);
        tk.send_action(TkAction::Reconfigure);
    }

    /// Resize, using the given closure to construct new widgets
    pub fn resize_with<F: Fn(usize) -> Box<dyn Widget>>(
        &mut self,
        tk: &mut dyn TkWindow,
        len: usize,
        f: F,
    ) {
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
