// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use super::{AxisInfo, SizeRules, Sizer};
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

/// A [`Sizer`] for grids supporting cell-spans
///
/// This implementation relies on the caller to provide storage for solver data.
pub struct FixedGridSolver<'a> {
    axis: AxisInfo,
    tk: &'a mut dyn TkWindow,
    widths: &'a mut [u32],
    heights: &'a mut [u32],
    width_rules: &'a mut [SizeRules],
    height_rules: &'a mut [SizeRules],
    span_rules: &'a mut [SizeRules],
}

impl<'a> FixedGridSolver<'a> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `tk`: `&dyn TkWindow` parameter passed into `size_rules`
    /// - `widths`: temporary storage of length *columns*, initialised to 0
    /// - `heights`: temporary storage of length *rows*, initialised to 0
    /// - `width_rules`: persistent storage of length *columns + 1*
    /// - `height_rules`: persistent storage of length *rows + 1*
    /// - `span_rules`: temporary storage of length *column spans*
    ///     (if `axis.horiz()`) or *row spans* (if `axis.vert()`)
    pub fn new(
        axis: AxisInfo,
        tk: &'a mut (dyn TkWindow + 'a),
        widths: &'a mut [u32],
        heights: &'a mut [u32],
        width_rules: &'a mut [SizeRules],
        height_rules: &'a mut [SizeRules],
        span_rules: &'a mut [SizeRules],
    ) -> Self {
        assert!(widths.len() + 1 == width_rules.len());
        assert!(heights.len() + 1 == height_rules.len());
        assert!(widths.iter().all(|w| *w == 0));
        assert!(heights.iter().all(|w| *w == 0));

        FixedGridSolver {
            axis,
            tk,
            widths,
            heights,
            width_rules,
            height_rules,
            span_rules,
        }
    }
}

impl<'a> Sizer for FixedGridSolver<'a> {
    type ChildInfo = GridChildInfo;

    fn prepare(&mut self) {
        if self.axis.has_fixed() {
            // TODO: cache this for use by set_rect?
            if self.axis.vert() {
                SizeRules::solve_seq(&mut self.widths, &self.width_rules, self.axis.other());
            } else {
                SizeRules::solve_seq(&mut self.heights, &self.height_rules, self.axis.other());
            }
        }

        if self.axis.horiz() {
            for n in 0..self.width_rules.len() {
                self.width_rules[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..self.height_rules.len() {
                self.height_rules[n] = SizeRules::EMPTY;
            }
        }
    }

    fn for_child<C: Layout>(&mut self, child_info: Self::ChildInfo, child: &mut C) {
        if self.axis.has_fixed() {
            if !self.axis.vert() {
                self.axis.set_size(
                    ((child_info.row + 1)..child_info.row_end)
                        .fold(self.heights[child_info.row], |h, i| h + self.heights[i]),
                );
            } else {
                self.axis.set_size(
                    ((child_info.col + 1)..child_info.col_end)
                        .fold(self.widths[child_info.col], |w, i| w + self.widths[i]),
                );
            }
        }
        let child_rules = child.size_rules(self.tk, self.axis);
        let rules = if !self.axis.vert() {
            if child_info.col_span_index == std::usize::MAX {
                &mut self.width_rules[child_info.col]
            } else {
                &mut self.span_rules[child_info.col_span_index]
            }
        } else {
            if child_info.row_span_index == std::usize::MAX {
                &mut self.height_rules[child_info.row]
            } else {
                &mut self.span_rules[child_info.row_span_index]
            }
        };
        *rules = rules.max(child_rules);
    }

    fn finish<Iter: Iterator<Item = (usize, usize, usize)>>(self, span_iter: Iter) -> SizeRules {
        let cols = self.width_rules.len() - 1;
        let rows = self.height_rules.len() - 1;

        let rules;
        if !self.axis.vert() {
            for span in span_iter {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| self.width_rules[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                self.width_rules[start].set_at_least_op_sub(self.span_rules[ind], sum);
            }

            rules = self.width_rules[0..cols]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            self.width_rules[cols] = rules;
        } else {
            for span in span_iter {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| self.height_rules[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                self.height_rules[start].set_at_least_op_sub(self.span_rules[ind], sum);
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
