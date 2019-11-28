// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use super::{AxisInfo, SizeRules, Sizer};
use crate::{Layout, TkWindow};

/// A [`Sizer`] for rows (and, without loss of generality, for columns).
///
/// This implementation relies on the caller to provide storage for solver data.
pub struct FixedRowSolver<'a> {
    // Generalisation implies that axis.vert() is incorrect
    axis: AxisInfo,
    tk: &'a mut dyn TkWindow,
    axis_is_vert: bool,
    rules: SizeRules,
    widths: &'a mut [u32],
    width_rules: &'a mut [SizeRules],
}

impl<'a> FixedRowSolver<'a> {
    /// Construct.
    ///
    /// - `vertical`: if true, this represents a column, not a row
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `tk`: `&dyn TkWindow` parameter passed into `size_rules`
    /// - `widths`: temporary storage of length *columns*, initialised to 0
    /// - `width_rules`: persistent storage of length *columns + 1*
    pub fn new(
        vertical: bool,
        axis: AxisInfo,
        tk: &'a mut (dyn TkWindow + 'a),
        widths: &'a mut [u32],
        width_rules: &'a mut [SizeRules],
    ) -> Self {
        assert!(widths.len() + 1 == width_rules.len());
        assert!(widths.iter().all(|w| *w == 0));

        let axis_is_vert = axis.vert() ^ vertical;
        FixedRowSolver {
            axis,
            tk,
            axis_is_vert,
            rules: SizeRules::EMPTY,
            widths,
            width_rules,
        }
    }
}

impl<'a> Sizer for FixedRowSolver<'a> {
    /// `ChildInfo` should contain the child index in the sequence
    type ChildInfo = usize;

    fn prepare(&mut self) {
        if self.axis.has_fixed() && self.axis_is_vert {
            // TODO: cache this for use by set_rect?
            SizeRules::solve_seq(&mut self.widths, self.width_rules, self.axis.other());
        }
    }

    fn for_child<C: Layout>(&mut self, child_info: Self::ChildInfo, child: &mut C) {
        if self.axis.has_fixed() && self.axis_is_vert {
            self.axis.set_size(self.widths[child_info]);
        }
        let child_rules = child.size_rules(self.tk, self.axis);
        if !self.axis_is_vert {
            self.width_rules[child_info] = child_rules;
            self.rules += child_rules;
        } else {
            self.rules = self.rules.max(child_rules);
        }
    }

    fn finish<Iter: Iterator<Item = (usize, usize, usize)>>(self, _: Iter) -> SizeRules {
        let cols = self.width_rules.len() - 1;
        if !self.axis_is_vert {
            self.width_rules[cols] = self.rules;
        }

        self.rules
    }
}
