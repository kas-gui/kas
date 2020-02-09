// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, GridStorage, Margins, RowTemp, RulesSetter, RulesSolver, SizeRules};
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

                let sum = (start..end)
                    .map(|n| storage.width_ref()[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                storage.width_mut()[start]
                    .set_at_least_op_sub(self.col_span_rules.as_ref()[ind], sum);
            }

            rules = storage.width_ref()[0..cols]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            storage.width_mut()[cols] = rules;
        } else {
            for span in row_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| storage.height_ref()[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                storage.height_mut()[start]
                    .set_at_least_op_sub(self.row_span_rules.as_ref()[ind], sum);
            }

            rules = storage.height_ref()[0..rows]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            storage.height_mut()[rows] = rules;
        }

        rules
    }
}

pub struct GridSetter<RT: RowTemp, CT: RowTemp, S: GridStorage> {
    widths: RT,
    heights: CT,
    col_pos: RT,
    row_pos: CT,
    pos: Coord,
    inter: Size,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> GridSetter<RT, CT, S> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `margins`: margin sizes
    /// - `storage`: reference to persistent storage
    pub fn new(
        mut rect: Rect,
        margins: Margins,
        (cols, rows): (usize, usize),
        storage: &mut S,
    ) -> Self {
        let mut widths = RT::default();
        let mut heights = CT::default();
        widths.set_len(cols);
        heights.set_len(rows);

        storage.set_width_len(cols + 1);
        storage.set_height_len(rows + 1);

        rect.pos += margins.first;
        rect.size -= margins.first + margins.last;
        let inter = margins.inter;

        SizeRules::solve_seq(widths.as_mut(), storage.width_ref(), rect.size.0);
        SizeRules::solve_seq(heights.as_mut(), storage.height_ref(), rect.size.1);

        let mut col_pos = RT::default();
        let mut row_pos = CT::default();
        col_pos.set_len(cols);
        row_pos.set_len(rows);
        let mut pos = 0;
        for n in 0..cols {
            col_pos.as_mut()[n] = pos;
            pos += widths.as_ref()[n] + inter.0;
        }
        pos = 0;
        for n in 0..rows {
            row_pos.as_mut()[n] = pos;
            pos += heights.as_ref()[n] + inter.1;
        }

        GridSetter {
            widths,
            heights,
            col_pos,
            row_pos,
            pos: rect.pos,
            inter,
            _s: Default::default(),
        }
    }
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> RulesSetter for GridSetter<RT, CT, S> {
    type Storage = S;
    type ChildInfo = GridChildInfo;

    fn child_rect(&mut self, info: Self::ChildInfo) -> Rect {
        let pos = self.pos
            + Coord(
                self.col_pos.as_ref()[info.col] as i32,
                self.row_pos.as_ref()[info.row] as i32,
            );

        let mut size = Size(
            self.inter.0 * (info.col_end - info.col - 1) as u32,
            self.inter.1 * (info.row_end - info.row - 1) as u32,
        );
        for n in info.col..info.col_end {
            size.0 += self.widths.as_ref()[n];
        }
        for n in info.row..info.row_end {
            size.1 += self.heights.as_ref()[n];
        }

        Rect { pos, size }
    }
}
