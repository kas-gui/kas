// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Miscellaneous solvers

use super::{AxisInfo, RulesSetter, RulesSolver, SizeRules};
use crate::geom::Rect;
use kas::AlignHints;

/// [`RulesSolver`] implementation for a fixed single-child layout
pub struct SingleSolver {
    axis: AxisInfo,
    rules: SizeRules,
}

impl SingleSolver {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `_dim`: direction and number of items
    /// - `_storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, _dim: (), _storage: &mut ()) -> Self {
        SingleSolver {
            axis,
            rules: SizeRules::EMPTY,
        }
    }
}

impl RulesSolver for SingleSolver {
    type Storage = ();
    type ChildInfo = ();

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        _storage: &mut Self::Storage,
        _child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
        self.rules = child_rules(self.axis);
    }

    fn finish(self, _storage: &mut Self::Storage) -> SizeRules {
        self.rules
    }
}

/// [`RulesSetter`] implementation for a fixed single-child layout
pub struct SingleSetter {
    rect: Rect,
}

impl SingleSetter {
    /// Construct
    ///
    /// All setter constructors take the following arguments:
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// -   `dim`: dimension information (specific to the setter, in this case
    ///     nothing)
    /// -   `align`: alignment hints
    /// -   `storage`: access to the solver's storage
    pub fn new(rect: Rect, _: (), _: AlignHints, _: &mut ()) -> Self {
        // NOTE: possibly we should apply alignment here, but we can't without
        // storing the ideal size for each dimension in the storage.
        // If we do, we should do the same for the other axis of RowSetter.
        SingleSetter { rect }
    }
}

impl RulesSetter for SingleSetter {
    type Storage = ();
    type ChildInfo = ();

    fn child_rect(&mut self, _: &mut Self::Storage, _: Self::ChildInfo) -> Rect {
        self.rect
    }

    fn maximal_rect_of(&mut self, _: &mut Self::Storage, _: Self::ChildInfo) -> Rect {
        self.rect
    }
}
