// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use super::{AxisInfo, RulesSolver, SizeRules};
use crate::{Layout, TkWindow};

/// Per-child information
pub struct GridChildInfo {
    /// Column index (first column when in a span)
    pub col: usize,
    /// One-past-last index of column span (`col_end = col + 1` without span)
    pub col_end: usize,
    /// Index in the list of all column spans (order is unimportant so long as
    /// each column span has a unique index).
    pub col_span_index: usize,
    /// Row index (first row when in a span)
    pub row: usize,
    /// One-past-last index of row span (`row_end = row + 1` without span)
    pub row_end: usize,
    /// Index in the list of all row spans (order is unimportant so long as
    /// each row span has a unique index).
    pub row_span_index: usize,
}

/// A [`RulesSolver`] for grids supporting cell-spans
///
/// This implementation relies on the caller to provide storage for solver data.
pub struct FixedGridSolver<'a, W, H, CSR, RSR> {
    axis: AxisInfo,
    tk: &'a mut dyn TkWindow,
    widths: W,
    heights: H,
    width_rules: &'a mut [SizeRules],
    height_rules: &'a mut [SizeRules],
    col_span_rules: CSR,
    row_span_rules: RSR,
}

impl<'a, W, H, CSR, RSR> FixedGridSolver<'a, W, H, CSR, RSR>
where
    W: Default + AsRef<[u32]> + AsMut<[u32]>,
    H: Default + AsRef<[u32]> + AsMut<[u32]>,
    CSR: Default,
    RSR: Default,
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `tk`: `&dyn TkWindow` parameter passed into `size_rules`
    /// - `width_rules`: persistent storage of length *columns + 1*
    /// - `height_rules`: persistent storage of length *rows + 1*
    pub fn new(
        axis: AxisInfo,
        tk: &'a mut (dyn TkWindow + 'a),
        width_rules: &'a mut [SizeRules],
        height_rules: &'a mut [SizeRules],
    ) -> Self {
        let widths = W::default();
        let heights = H::default();
        let col_span_rules = CSR::default();
        let row_span_rules = RSR::default();

        assert!(widths.as_ref().len() + 1 == width_rules.len());
        assert!(heights.as_ref().len() + 1 == height_rules.len());
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        assert!(heights.as_ref().iter().all(|w| *w == 0));

        let mut solver = FixedGridSolver {
            axis,
            tk,
            widths,
            heights,
            width_rules,
            height_rules,
            col_span_rules,
            row_span_rules,
        };
        solver.prepare();
        solver
    }

    fn prepare(&mut self) {
        if self.axis.has_fixed {
            // TODO: cache this for use by set_rect?
            if self.axis.vertical {
                SizeRules::solve_seq(
                    self.widths.as_mut(),
                    &self.width_rules,
                    self.axis.other_axis,
                );
            } else {
                SizeRules::solve_seq(
                    self.heights.as_mut(),
                    &self.height_rules,
                    self.axis.other_axis,
                );
            }
        }

        if !self.axis.vertical {
            for n in 0..self.width_rules.len() {
                self.width_rules[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..self.height_rules.len() {
                self.height_rules[n] = SizeRules::EMPTY;
            }
        }
    }
}

impl<'a, W, H, CSR, RSR> RulesSolver for FixedGridSolver<'a, W, H, CSR, RSR>
where
    W: AsRef<[u32]>,
    H: AsRef<[u32]>,
    CSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    RSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    type ChildInfo = GridChildInfo;

    fn for_child<C: Layout>(&mut self, child_info: Self::ChildInfo, child: &mut C) {
        if self.axis.has_fixed {
            if !self.axis.vertical {
                self.axis.other_axis = ((child_info.row + 1)..child_info.row_end)
                    .fold(self.heights.as_ref()[child_info.row], |h, i| {
                        h + self.heights.as_ref()[i]
                    });
            } else {
                self.axis.other_axis = ((child_info.col + 1)..child_info.col_end)
                    .fold(self.widths.as_ref()[child_info.col], |w, i| {
                        w + self.widths.as_ref()[i]
                    });
            }
        }
        let child_rules = child.size_rules(self.tk, self.axis);
        let rules = if !self.axis.vertical {
            if child_info.col_span_index == std::usize::MAX {
                &mut self.width_rules[child_info.col]
            } else {
                &mut self.col_span_rules.as_mut()[child_info.col_span_index]
            }
        } else {
            if child_info.row_span_index == std::usize::MAX {
                &mut self.height_rules[child_info.row]
            } else {
                &mut self.row_span_rules.as_mut()[child_info.row_span_index]
            }
        };
        *rules = rules.max(child_rules);
    }

    fn finish<ColIter, RowIter>(self, col_spans: ColIter, row_spans: RowIter) -> SizeRules
    where
        ColIter: Iterator<Item = (usize, usize, usize)>,
        RowIter: Iterator<Item = (usize, usize, usize)>,
    {
        let cols = self.width_rules.len() - 1;
        let rows = self.height_rules.len() - 1;

        let rules;
        if !self.axis.vertical {
            for span in col_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| self.width_rules[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                self.width_rules[start].set_at_least_op_sub(self.col_span_rules.as_ref()[ind], sum);
            }

            rules = self.width_rules[0..cols]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            self.width_rules[cols] = rules;
        } else {
            for span in row_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| self.height_rules[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                self.height_rules[start]
                    .set_at_least_op_sub(self.row_span_rules.as_ref()[ind], sum);
            }

            rules = self.height_rules[0..rows]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            self.height_rules[rows] = rules;
        }

        rules
    }
}
