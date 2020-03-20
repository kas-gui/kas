// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver

use log::trace;
use std::fmt;

use super::{AxisInfo, SizeRules};
use crate::draw::SizeHandle;
use crate::geom::{Coord, Rect, Size};
use crate::Direction::{Horizontal, Vertical};
use crate::{AlignHints, Layout, WidgetConfig};

/// A [`SizeRules`] solver for layouts
///
/// Typically, a solver is invoked twice, once for each axis, before the
/// corresponding [`RulesSetter`] is invoked. This is managed by [`solve`].
///
/// Implementations require access to storage able to persist between multiple
/// solver runs and a subsequent setter run. This storage is of type
/// [`RulesSolver::Storage`] and is passed via reference to the constructor.
pub trait RulesSolver {
    /// Type of storage
    type Storage: Clone;

    /// Type required by [`RulesSolver::for_child`] (see implementation documentation)
    type ChildInfo;

    /// Called once for each child. For most layouts the order is important.
    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    );

    /// Called at the end to output [`SizeRules`].
    ///
    /// Note that this does not include margins!
    fn finish(self, storage: &mut Self::Storage) -> SizeRules;
}

/// Resolves a [`RulesSolver`] solution for each child
pub trait RulesSetter {
    /// Type of storage
    type Storage: Clone;

    /// Type required by [`RulesSolver::for_child`] (see implementation documentation)
    type ChildInfo;

    /// Called once for each child. For most layouts the order is important.
    fn child_rect(&mut self, child_info: Self::ChildInfo) -> Rect;
}

/// Calculate required size of widget
///
/// Return min and ideal sizes.
pub fn solve<L: Layout>(widget: &mut L, size_handle: &mut dyn SizeHandle) -> (Size, Size) {
    // We call size_rules not because we want the result, but because our
    // spec requires that we do so before calling set_rect.
    let w = widget.size_rules(size_handle, AxisInfo::new(Horizontal, None));
    let h = widget.size_rules(size_handle, AxisInfo::new(Vertical, Some(w.ideal_size())));

    let min = Size(w.min_size(), h.min_size());
    let ideal = Size(w.ideal_size(), h.ideal_size());
    trace!("layout::solve: min={:?}, ideal={:?}", min, ideal);
    (min, ideal)
}

/// Solve and assign widget layout
///
/// Return min and ideal sizes.
pub fn solve_and_set<L: Layout>(
    widget: &mut L,
    size_handle: &mut dyn SizeHandle,
    size: Size,
) -> (Size, Size) {
    // We call size_rules not because we want the result, but because our
    // spec requires that we do so before calling set_rect.
    let w = widget.size_rules(size_handle, AxisInfo::new(Horizontal, None));
    let m = w.margins();
    let x = m.0 as i32;
    let width = size.0 - (m.0 + m.1) as u32;

    let h = widget.size_rules(size_handle, AxisInfo::new(Vertical, Some(width)));
    let m = h.margins();
    let y = m.0 as i32;
    let height = size.1 - (m.0 + m.1) as u32;

    let pos = Coord(x, y);
    let size = Size(width, height);
    widget.set_rect(size_handle, Rect { pos, size }, AlignHints::NONE);

    trace!(
        "layout::solve_and_set for size={:?} has rules {:?}, {:?} and hierarchy:{}",
        size,
        w,
        h,
        WidgetHeirarchy(widget, 0),
    );

    let min = Size(w.min_size(), h.min_size());
    let ideal = Size(w.ideal_size(), h.ideal_size());
    (min, ideal)
}

struct WidgetHeirarchy<'a>(&'a dyn WidgetConfig, usize);
impl<'a> fmt::Display for WidgetHeirarchy<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "\n{}{}\t{}\tpos={:?}\tsize={:?}",
            "- ".repeat(self.1),
            self.0.id(),
            self.0.widget_name(),
            self.0.rect().pos,
            self.0.rect().size,
        )?;

        for i in 0..self.0.len() {
            WidgetHeirarchy(self.0.get(i).unwrap(), self.1 + 1).fmt(f)?;
        }
        Ok(())
    }
}
