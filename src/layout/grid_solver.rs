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
    pub col: u32,
    /// One-past-last index of column span (`col_end = col + 1` without span)
    pub col_end: u32,
    /// Row index (first row when in a span)
    pub row: u32,
    /// One-past-last index of row span (`row_end = row + 1` without span)
    pub row_end: u32,
}

/// A [`RulesSolver`] for grids supporting cell-spans
///
/// This implementation relies on the caller to provide storage for solver data.
pub struct GridSolver<RT, CT, CSR, RSR, S: GridStorage> {
    axis: AxisInfo,
    widths: RT,
    heights: CT,
    col_spans: CSR,
    row_spans: RSR,
    next_col_span: usize,
    next_row_span: usize,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, CSR: Default, RSR: Default, S: GridStorage>
    GridSolver<RT, CT, CSR, RSR, S>
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `(cols, rows)`: number of columns and rows
    /// - `storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, (cols, rows): (usize, usize), storage: &mut S) -> Self {
        let mut widths = RT::default();
        let mut heights = CT::default();
        widths.set_len(cols);
        heights.set_len(rows);
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        assert!(heights.as_ref().iter().all(|w| *w == 0));

        let col_spans = CSR::default();
        let row_spans = RSR::default();

        storage.set_dims(cols + 1, rows + 1);

        let mut solver = GridSolver {
            axis,
            widths,
            heights,
            col_spans,
            row_spans,
            next_col_span: 0,
            next_row_span: 0,
            _s: Default::default(),
        };
        solver.prepare(storage);
        solver
    }

    fn prepare(&mut self, storage: &mut S) {
        if self.axis.has_fixed {
            // TODO: cache this for use by set_rect?
            if self.axis.is_vertical() {
                SizeRules::solve_seq(self.widths.as_mut(), storage.widths(), self.axis.other_axis);
            } else {
                SizeRules::solve_seq(
                    self.heights.as_mut(),
                    storage.heights(),
                    self.axis.other_axis,
                );
            }
        }

        if self.axis.is_horizontal() {
            for n in 0..storage.widths().len() {
                storage.widths_mut()[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..storage.heights().len() {
                storage.heights_mut()[n] = SizeRules::EMPTY;
            }
        }
    }
}

impl<RT: RowTemp, CT: RowTemp, CSR, RSR, S: GridStorage> RulesSolver
    for GridSolver<RT, CT, CSR, RSR, S>
where
    CSR: AsRef<[(SizeRules, u32, u32)]> + AsMut<[(SizeRules, u32, u32)]>,
    RSR: AsRef<[(SizeRules, u32, u32)]> + AsMut<[(SizeRules, u32, u32)]>,
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
                    .fold(self.heights.as_ref()[child_info.row as usize], |h, i| {
                        h + self.heights.as_ref()[i as usize]
                    });
            } else {
                self.axis.other_axis = ((child_info.col + 1)..child_info.col_end)
                    .fold(self.widths.as_ref()[child_info.col as usize], |w, i| {
                        w + self.widths.as_ref()[i as usize]
                    });
            }
        }
        let child_rules = child_rules(self.axis);
        if self.axis.is_horizontal() {
            if child_info.col_end > child_info.col + 1 {
                let span = &mut self.col_spans.as_mut()[self.next_col_span];
                span.0.max_with(child_rules);
                span.1 = child_info.col;
                span.2 = child_info.col_end;
                self.next_col_span += 1;
            } else {
                storage.widths_mut()[child_info.col as usize].max_with(child_rules);
            }
        } else {
            if child_info.row_end > child_info.row + 1 {
                let span = &mut self.row_spans.as_mut()[self.next_row_span];
                span.0.max_with(child_rules);
                span.1 = child_info.row;
                span.2 = child_info.row_end;
                self.next_row_span += 1;
            } else {
                storage.heights_mut()[child_info.row as usize].max_with(child_rules);
            }
        };
    }

    fn finish(mut self, storage: &mut Self::Storage) -> SizeRules {
        fn calculate(
            cols: usize,
            widths: &mut [SizeRules],
            spans: &mut [(SizeRules, u32, u32)],
        ) -> SizeRules {
            // For each span, we ensure cell widths are sufficiently large.
            // Note that this distribution may not be optimal in the case of
            // partially-overlapping spans; since those are rare this case
            // remains unsolved for now (in any case, all spans will be
            // sufficiently large, but some space may be wasted).

            for span in spans {
                let rules = span.0;
                let begin = span.1 as usize;
                let end = span.2 as usize;
                rules.distribute_span_over(&mut widths[begin..end]);
            }

            let rules = widths[0..cols].iter().sum();
            widths[cols] = rules;
            rules
        }

        if self.axis.is_horizontal() {
            let cols = storage.widths().len() - 1;
            calculate(cols, storage.widths_mut(), self.col_spans.as_mut())
        } else {
            let rows = storage.heights().len() - 1;
            calculate(rows, storage.heights_mut(), self.row_spans.as_mut())
        }
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
    /// - `rect`: target [`Rect`]
    /// - `(cols, rows)`: number of columns and rows
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

        storage.set_dims(cols + 1, rows + 1);

        SizeRules::solve_seq(widths.as_mut(), storage.widths(), rect.size.0);
        w_offsets.as_mut()[0] = 0;
        for i in 1..w_offsets.as_ref().len() {
            let i1 = i - 1;
            let m1 = storage.widths()[i1].margins().1;
            let m0 = storage.widths()[i].margins().0;
            w_offsets.as_mut()[i] =
                w_offsets.as_ref()[i1] + widths.as_ref()[i1] + m1.max(m0) as u32;
        }

        SizeRules::solve_seq(heights.as_mut(), storage.heights(), rect.size.1);
        h_offsets.as_mut()[0] = 0;
        for i in 1..h_offsets.as_ref().len() {
            let i1 = i - 1;
            let m1 = storage.heights()[i1].margins().1;
            let m0 = storage.heights()[i].margins().0;
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
        let x = self.w_offsets.as_ref()[info.col as usize] as i32;
        let y = self.h_offsets.as_ref()[info.row as usize] as i32;
        let pos = self.pos + Coord(x, y);

        let i1 = info.col_end as usize - 1;
        let w = self.widths.as_ref()[i1] + self.w_offsets.as_ref()[i1]
            - self.w_offsets.as_ref()[info.col as usize];
        let i1 = info.row_end as usize - 1;
        let h = self.heights.as_ref()[i1] + self.h_offsets.as_ref()[i1]
            - self.h_offsets.as_ref()[info.row as usize];
        let size = Size(w, h);

        Rect { pos, size }
    }
}
