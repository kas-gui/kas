// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, GridStorage, RowTemp, RulesSetter, RulesSolver, SizeRules};
use crate::geom::{Coord, Rect, Size};

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
pub struct GridSolver<RT, CT, CSR, RSR, S: GridStorage> {
    axis: AxisInfo,
    widths: RT,
    heights: CT,
    col_span_rules: CSR,
    row_span_rules: RSR,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, CSR: Default, RSR: Default, S: GridStorage>
    GridSolver<RT, CT, CSR, RSR, S>
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, (cols, rows): (usize, usize), storage: &mut S) -> Self {
        let mut widths = RT::default();
        let mut heights = CT::default();
        widths.set_len(cols);
        heights.set_len(rows);
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        assert!(heights.as_ref().iter().all(|w| *w == 0));

        let col_span_rules = CSR::default();
        let row_span_rules = RSR::default();

        storage.set_width_len(cols + 1);
        storage.set_height_len(rows + 1);

        let mut solver = GridSolver {
            axis,
            widths,
            heights,
            col_span_rules,
            row_span_rules,
            _s: Default::default(),
        };
        solver.prepare(storage);
        solver
    }

    fn prepare(&mut self, storage: &mut S) {
        if self.axis.has_fixed {
            // TODO: cache this for use by set_rect?
            if self.axis.is_vertical() {
                SizeRules::solve_seq(
                    self.widths.as_mut(),
                    storage.width_ref(),
                    self.axis.other_axis,
                );
            } else {
                SizeRules::solve_seq(
                    self.heights.as_mut(),
                    storage.height_ref(),
                    self.axis.other_axis,
                );
            }
        }

        if self.axis.is_horizontal() {
            for n in 0..storage.width_ref().len() {
                storage.width_mut()[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..storage.height_ref().len() {
                storage.height_mut()[n] = SizeRules::EMPTY;
            }
        }
    }
}

impl<RT: RowTemp, CT: RowTemp, CSR, RSR, S: GridStorage> RulesSolver
    for GridSolver<RT, CT, CSR, RSR, S>
where
    CSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    RSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    type Storage = S;
    type ChildInfo = GridChildInfo;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.axis.has_fixed {
            if self.axis.is_horizontal() {
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
        let child_rules = child_rules(self.axis);
        let rules = if self.axis.is_horizontal() {
            if child_info.col_span_index == std::usize::MAX {
                &mut storage.width_mut()[child_info.col]
            } else {
                &mut self.col_span_rules.as_mut()[child_info.col_span_index]
            }
        } else {
            if child_info.row_span_index == std::usize::MAX {
                &mut storage.height_mut()[child_info.row]
            } else {
                &mut self.row_span_rules.as_mut()[child_info.row_span_index]
            }
        };
        *rules = rules.max(child_rules);
    }

    fn finish<ColIter, RowIter>(
        self,
        storage: &mut Self::Storage,
        col_spans: ColIter,
        row_spans: RowIter,
    ) -> SizeRules
    where
        ColIter: Iterator<Item = (usize, usize, usize)>,
        RowIter: Iterator<Item = (usize, usize, usize)>,
    {
        let cols = storage.width_ref().len() - 1;
        let rows = storage.height_ref().len() - 1;

        let rules;
        if self.axis.is_horizontal() {
            for span in col_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end).map(|n| storage.width_ref()[n]).sum();
                storage.width_mut()[start]
                    .set_at_least_op_sub(self.col_span_rules.as_ref()[ind], sum);
            }

            rules = storage.width_ref()[0..cols].iter().sum();
            storage.width_mut()[cols] = rules;
        } else {
            for span in row_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end).map(|n| storage.height_ref()[n]).sum();
                storage.height_mut()[start]
                    .set_at_least_op_sub(self.row_span_rules.as_ref()[ind], sum);
            }

            rules = storage.height_ref()[0..rows].iter().sum();
            storage.height_mut()[rows] = rules;
        }

        rules
    }
}

pub struct GridSetter<RT: RowTemp, CT: RowTemp, S: GridStorage> {
    widths: RT,
    heights: CT,
    w_offsets: RT,
    h_offsets: CT,
    pos: Coord,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> GridSetter<RT, CT, S> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `storage`: reference to persistent storage
    pub fn new(rect: Rect, (cols, rows): (usize, usize), storage: &mut S) -> Self {
        let mut widths = RT::default();
        widths.set_len(cols);
        let mut heights = CT::default();
        heights.set_len(rows);
        let mut w_offsets = RT::default();
        w_offsets.set_len(cols);
        let mut h_offsets = CT::default();
        h_offsets.set_len(rows);

        storage.set_width_len(cols + 1);
        storage.set_height_len(rows + 1);

        SizeRules::solve_seq(widths.as_mut(), storage.width_ref(), rect.size.0);
        w_offsets.as_mut()[0] = 0;
        for i in 1..w_offsets.as_ref().len() {
            let i1 = i - 1;
            let m1 = storage.width_ref()[i1].margins().1;
            let m0 = storage.width_ref()[i].margins().0;
            w_offsets.as_mut()[i] =
                w_offsets.as_ref()[i1] + widths.as_ref()[i1] + m1.max(m0) as u32;
        }

        SizeRules::solve_seq(heights.as_mut(), storage.height_ref(), rect.size.1);
        h_offsets.as_mut()[0] = 0;
        for i in 1..h_offsets.as_ref().len() {
            let i1 = i - 1;
            let m1 = storage.height_ref()[i1].margins().1;
            let m0 = storage.height_ref()[i].margins().0;
            h_offsets.as_mut()[i] =
                h_offsets.as_ref()[i1] + heights.as_ref()[i1] + m1.max(m0) as u32;
        }

        GridSetter {
            widths,
            heights,
            w_offsets,
            h_offsets,
            pos: rect.pos,
            _s: Default::default(),
        }
    }
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> RulesSetter for GridSetter<RT, CT, S> {
    type Storage = S;
    type ChildInfo = GridChildInfo;

    fn child_rect(&mut self, info: Self::ChildInfo) -> Rect {
        let x = self.w_offsets.as_ref()[info.col] as i32;
        let y = self.h_offsets.as_ref()[info.row] as i32;
        let pos = self.pos + Coord(x, y);

        let i1 = info.col_end - 1;
        let w = self.widths.as_ref()[i1] + self.w_offsets.as_ref()[i1]
            - self.w_offsets.as_ref()[info.col];
        let i1 = info.row_end - 1;
        let h = self.heights.as_ref()[i1] + self.h_offsets.as_ref()[i1]
            - self.h_offsets.as_ref()[info.row];
        let size = Size(w, h);

        Rect { pos, size }
    }
}
