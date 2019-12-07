// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver

use super::{AxisInfo, SizeRules};
use crate::geom::{Coord, Rect, Size};
use crate::{TkWindow, Widget};

pub trait Storage {}

/// A [`SizeRules`] solver for layouts
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
    fn finish<ColIter, RowIter>(
        self,
        storage: &mut Self::Storage,
        col_spans: ColIter,
        row_spans: RowIter,
    ) -> SizeRules
    where
        ColIter: Iterator<Item = (usize, usize, usize)>,
        RowIter: Iterator<Item = (usize, usize, usize)>;
}

/// Tool to solve for a `Rect` over child widgets
pub trait RulesSetter {
    /// Type of storage
    type Storage: Clone;

    /// Type required by [`RulesSolver::for_child`] (see implementation documentation)
    type ChildInfo;

    /// Called once for each child. For most layouts the order is important.
    fn child_rect(&mut self, child_info: Self::ChildInfo) -> Rect;
}

/// Solve `widget` for `SizeRules` on both axes, horizontal first.
pub fn solve<L: Widget>(widget: &mut L, tk: &mut dyn TkWindow, size: Size) {
    // We call size_rules not because we want the result, but because our
    // spec requires that we do so before calling set_rect.
    tk.with_size_handle(&mut |size_handle| {
        let _w = widget.size_rules(size_handle, AxisInfo::new(false, None));
        let _h = widget.size_rules(size_handle, AxisInfo::new(true, Some(size.0)));

        let pos = Coord(0, 0);
        widget.set_rect(size_handle, Rect { pos, size });

        // println!("Window size:\t{:?}", size);
        // println!("Width rules:\t{:?}", _w);
        // println!("Height rules:\t{:?}", _h);
        // widget.print_hierarchy(0);
    });
}
