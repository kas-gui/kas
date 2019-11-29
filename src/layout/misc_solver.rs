// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Miscellaneous solvers

use super::{AxisInfo, Margins, RulesSetter, RulesSolver, SizeRules};
use crate::geom::Rect;

/// Dummy implementation
impl RulesSolver for () {
    type Storage = ();
    type ChildInfo = ();

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        _storage: &mut Self::Storage,
        _child_info: Self::ChildInfo,
        _child_rules: CR,
    ) {
    }

    fn finish<ColIter, RowIter>(
        self,
        _storage: &mut Self::Storage,
        _col_spans: ColIter,
        _row_spans: RowIter,
    ) -> SizeRules
    where
        ColIter: Iterator<Item = (usize, usize, usize)>,
        RowIter: Iterator<Item = (usize, usize, usize)>,
    {
        unimplemented!()
    }
}

/// Dummy implementation
impl RulesSetter for () {
    type Storage = ();
    type ChildInfo = ();

    fn child_rect(&mut self, _child_info: Self::ChildInfo) -> Rect {
        unimplemented!()
    }
}

/// [`RulesSetter`] implementation for a fixed single-child layout
///
/// Note: there is no `SingleSolver` ([`RulesSolver`] implementation) for this
/// case; such an implementation could not do anything more than returning the
/// result of `self.child.size_rules(...)`.
pub struct SingleSetter {
    crect: Rect,
}

impl SingleSetter {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `margins`: margin sizes
    /// - `storage`: irrelevent, but included for consistency
    pub fn new(mut rect: Rect, margins: Margins, _storage: &mut ()) -> Self {
        rect.pos += margins.first;
        rect.size -= margins.first + margins.last;
        let crect = rect;

        SingleSetter { crect }
    }
}

impl RulesSetter for SingleSetter {
    type Storage = ();
    type ChildInfo = ();

    fn child_rect(&mut self, _child_info: Self::ChildInfo) -> Rect {
        self.crect
    }
}
