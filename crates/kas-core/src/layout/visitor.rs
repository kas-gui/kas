// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout visitor

use super::{AlignHints, AxisInfo, RulesSetter, RulesSolver, SizeRules};
use super::{RowSetter, RowSolver, RowStorage};
use crate::draw::SizeHandle;
use crate::event::Manager;
use crate::geom::Rect;
use crate::{dir::Directional, WidgetConfig};
use std::iter::ExactSizeIterator;

/// Implementation helper for layout of children
pub trait Visitor {
    /// Get size rules for the given axis
    fn size_rules(self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules;

    /// Apply a given `rect` to self
    fn set_rect(self, mgr: &mut Manager, rect: Rect, align: AlignHints);
}

/// Implement row/column layout for children
pub struct List<'a, L: RowStorage, D: Directional, I>
where
    I: ExactSizeIterator<Item = (usize, &'a mut dyn WidgetConfig)>,
{
    data: &'a mut L,
    direction: D,
    children: I,
}

impl<'a, L: RowStorage, D: Directional, I> List<'a, L, D, I>
where
    I: ExactSizeIterator<Item = (usize, &'a mut dyn WidgetConfig)>,
{
    pub fn new(data: &'a mut L, direction: D, children: I) -> Self {
        List {
            data,
            direction,
            children,
        }
    }
}

impl<'a, L: RowStorage, D: Directional, I> Visitor for List<'a, L, D, I>
where
    I: ExactSizeIterator<Item = (usize, &'a mut dyn WidgetConfig)>,
{
    fn size_rules(self, sh: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let dim = (self.direction, self.children.len());
        let mut solver = RowSolver::new(axis, dim, self.data);
        for (n, child) in self.children {
            solver.for_child(self.data, n, |axis| child.size_rules(sh, axis));
        }
        solver.finish(self.data)
    }

    fn set_rect(self, mgr: &mut Manager, rect: Rect, align: AlignHints) {
        let dim = (self.direction, self.children.len());
        let mut setter = RowSetter::<D, Vec<i32>, _>::new(rect, dim, align, self.data);

        for (n, child) in self.children {
            child.set_rect(mgr, setter.child_rect(self.data, n), align);
        }
    }
}
