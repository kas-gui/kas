// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, GridStorage, RowTemp, RulesSetter, RulesSolver, SizeRules};
use crate::geom::{Coord, Rect, Size};
use kas::{Align, AlignHints};

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
pub struct GridSolver<CSR, RSR, S: GridStorage> {
    axis: AxisInfo,
    col_spans: CSR,
    row_spans: RSR,
    next_col_span: usize,
    next_row_span: usize,
    _s: PhantomData<S>,
}

impl<CSR: Default, RSR: Default, S: GridStorage> GridSolver<CSR, RSR, S> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `(cols, rows)`: number of columns and rows
    /// - `storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, (cols, rows): (usize, usize), storage: &mut S) -> Self {
        let col_spans = CSR::default();
        let row_spans = RSR::default();

        storage.set_dims(cols, rows);

        let mut solver = GridSolver {
            axis,
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
            if self.axis.is_vertical() {
                let (rules, widths) = storage.rules_and_widths();
                SizeRules::solve_seq_total(widths, rules, self.axis.other_axis);
            } else {
                let (rules, heights) = storage.rules_and_heights();
                SizeRules::solve_seq_total(heights, rules, self.axis.other_axis);
            }
        }

        if self.axis.is_horizontal() {
            for n in 0..storage.width_rules().len() {
                storage.width_rules()[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..storage.height_rules().len() {
                storage.height_rules()[n] = SizeRules::EMPTY;
            }
        }
    }
}

impl<CSR, RSR, S: GridStorage> RulesSolver for GridSolver<CSR, RSR, S>
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
                    .fold(storage.heights()[child_info.row as usize], |h, i| {
                        h + storage.heights()[i as usize]
                    });
            } else {
                self.axis.other_axis = ((child_info.col + 1)..child_info.col_end)
                    .fold(storage.widths()[child_info.col as usize], |w, i| {
                        w + storage.widths()[i as usize]
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
                storage.width_rules()[child_info.col as usize].max_with(child_rules);
            }
        } else {
            if child_info.row_end > child_info.row + 1 {
                let span = &mut self.row_spans.as_mut()[self.next_row_span];
                span.0.max_with(child_rules);
                span.1 = child_info.row;
                span.2 = child_info.row_end;
                self.next_row_span += 1;
            } else {
                storage.height_rules()[child_info.row as usize].max_with(child_rules);
            }
        };
    }

    fn finish(mut self, storage: &mut Self::Storage) -> SizeRules {
        fn calculate(
            cols: usize,
            widths: &mut [SizeRules],
            spans: &mut [(SizeRules, u32, u32)],
        ) -> SizeRules {
            // spans: &mut [(rules, begin, end)]

            // We merge all overlapping spans in arbitrary order.
            let (mut i, mut j) = (0, 1);
            let mut len = spans.len();
            while j < len {
                let (first, second) = if spans[i].1 <= spans[j].1 {
                    (i, j)
                } else {
                    (j, i)
                };
                let first_end = spans[first].2 as usize;
                let second_begin = spans[second].1 as usize;
                if first_end <= second_begin {
                    j += 1;
                    if j >= len {
                        i += 1;
                        j = i + 1;
                    }
                    continue;
                }

                // Internal margins would be lost; handle those first.
                widths[second_begin].include_margins((spans[second].0.margins().0, 0));
                widths[first_end - 1].include_margins((0, spans[first].0.margins().1));

                let overlap_sum = widths[second_begin..first_end].iter().sum();
                spans[first].0.sub_add(overlap_sum, spans[second].0);

                spans.swap(second, len - 1);
                len -= 1;
            }

            // We are left with non-overlapping spans.
            // For each span, we ensure cell widths are sufficiently large.
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
            let cols = storage.width_rules().len() - 1;
            calculate(cols, storage.width_rules(), self.col_spans.as_mut())
        } else {
            let rows = storage.height_rules().len() - 1;
            calculate(rows, storage.height_rules(), self.row_spans.as_mut())
        }
    }
}

pub struct GridSetter<RT: RowTemp, CT: RowTemp, S: GridStorage> {
    w_offsets: RT,
    h_offsets: CT,
    pos: Coord,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> GridSetter<RT, CT, S> {
    /// Construct
    ///
    /// All setter constructors take the following arguments:
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// -   `dim`: dimension information (specific to the setter, in this case
    ///     number of columns and rows)
    /// -   `align`: alignment hints
    /// -   `storage`: access to the solver's storage
    pub fn new(
        rect: Rect,
        (cols, rows): (usize, usize),
        align: AlignHints,
        storage: &mut S,
    ) -> Self {
        let mut w_offsets = RT::default();
        w_offsets.set_len(cols);
        let mut h_offsets = CT::default();
        h_offsets.set_len(rows);

        storage.set_dims(cols, rows);

        if cols > 0 {
            let align = align.horiz.unwrap_or(Align::Stretch);
            let (rules, widths) = storage.rules_and_widths();
            let ideal = rules[cols].ideal_size();

            w_offsets.as_mut()[0] = 0;
            if align != Align::Stretch && rect.size.0 > ideal {
                let extra = rect.size.0 - ideal;
                w_offsets.as_mut()[0] = match align {
                    Align::Begin | Align::Stretch => 0,
                    Align::Centre => extra / 2,
                    Align::End => extra,
                };
            }

            SizeRules::solve_seq_total(widths, rules, rect.size.0);
            for i in 1..w_offsets.as_mut().len() {
                let i1 = i - 1;
                let m1 = storage.width_rules()[i1].margins().1;
                let m0 = storage.width_rules()[i].margins().0;
                w_offsets.as_mut()[i] =
                    w_offsets.as_mut()[i1] + storage.widths()[i1] + m1.max(m0) as u32;
            }
        }

        if rows > 0 {
            let align = align.vert.unwrap_or(Align::Stretch);
            let (rules, heights) = storage.rules_and_heights();
            let ideal = rules[rows].ideal_size();

            h_offsets.as_mut()[0] = 0;
            if align != Align::Stretch && rect.size.1 > ideal {
                let extra = rect.size.1 - ideal;
                h_offsets.as_mut()[0] = match align {
                    Align::Begin | Align::Stretch => 0,
                    Align::Centre => extra / 2,
                    Align::End => extra,
                };
            }

            SizeRules::solve_seq_total(heights, rules, rect.size.1);
            for i in 1..h_offsets.as_mut().len() {
                let i1 = i - 1;
                let m1 = storage.height_rules()[i1].margins().1;
                let m0 = storage.height_rules()[i].margins().0;
                h_offsets.as_mut()[i] =
                    h_offsets.as_mut()[i1] + storage.heights()[i1] + m1.max(m0) as u32;
            }
        }

        GridSetter {
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

    fn child_rect(&mut self, storage: &mut Self::Storage, info: Self::ChildInfo) -> Rect {
        let x = self.w_offsets.as_mut()[info.col as usize] as i32;
        let y = self.h_offsets.as_mut()[info.row as usize] as i32;
        let pos = self.pos + Coord(x, y);

        let i1 = info.col_end as usize - 1;
        let w = storage.widths()[i1] + self.w_offsets.as_mut()[i1]
            - self.w_offsets.as_mut()[info.col as usize];
        let i1 = info.row_end as usize - 1;
        let h = storage.heights()[i1] + self.h_offsets.as_mut()[i1]
            - self.h_offsets.as_mut()[info.row as usize];
        let size = Size(w, h);

        Rect { pos, size }
    }

    fn maximal_rect_of(&mut self, _storage: &mut Self::Storage, _index: Self::ChildInfo) -> Rect {
        unimplemented!()
    }
}
