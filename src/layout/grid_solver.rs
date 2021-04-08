// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{Align, AlignHints, AxisInfo, SizeRules};
use super::{GridStorage, RowTemp, RulesSetter, RulesSolver};
use crate::cast::Conv;
use crate::geom::{Coord, Offset, Rect, Size};

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
    /// Argument order is consistent with other [`RulesSolver`]s.
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
                let (widths, rules, total) = storage.widths_rules_total();
                SizeRules::solve_seq_total(widths, rules, total, self.axis.other_axis);
            } else {
                let (heights, rules, total) = storage.heights_rules_total();
                SizeRules::solve_seq_total(heights, rules, total, self.axis.other_axis);
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
                    .fold(storage.heights()[usize::conv(child_info.row)], |h, i| {
                        h + storage.heights()[usize::conv(i)]
                    });
            } else {
                self.axis.other_axis = ((child_info.col + 1)..child_info.col_end)
                    .fold(storage.widths()[usize::conv(child_info.col)], |w, i| {
                        w + storage.widths()[usize::conv(i)]
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
                storage.width_rules()[usize::conv(child_info.col)].max_with(child_rules);
            }
        } else {
            if child_info.row_end > child_info.row + 1 {
                let span = &mut self.row_spans.as_mut()[self.next_row_span];
                span.0.max_with(child_rules);
                span.1 = child_info.row;
                span.2 = child_info.row_end;
                self.next_row_span += 1;
            } else {
                storage.height_rules()[usize::conv(child_info.row)].max_with(child_rules);
            }
        };
    }

    fn finish(mut self, storage: &mut Self::Storage) -> SizeRules {
        fn calculate(widths: &mut [SizeRules], spans: &mut [(SizeRules, u32, u32)]) -> SizeRules {
            // spans: &mut [(rules, begin, end)]

            // To avoid losing Stretch, we distribute this first
            const BASE_WEIGHT: u32 = 100;
            const SPAN_WEIGHT: u32 = 10;
            let mut scores: Vec<u32> = widths
                .iter()
                .map(|w| w.stretch() as u32 * BASE_WEIGHT)
                .collect();
            for span in spans.iter() {
                let w = span.0.stretch() as u32 * SPAN_WEIGHT;
                for score in &mut scores[(usize::conv(span.1))..(usize::conv(span.2))] {
                    *score += w;
                }
            }
            for span in spans.iter() {
                let range = (usize::conv(span.1))..(usize::conv(span.2));
                span.0
                    .distribute_stretch_over_by(&mut widths[range.clone()], &scores[range]);
            }

            // We merge all overlapping spans in arbitrary order.
            let (mut i, mut j) = (0, 1);
            let mut len = spans.len();
            while j < len {
                let (first, second) = if spans[i].1 <= spans[j].1 {
                    (i, j)
                } else {
                    (j, i)
                };
                let first_end = usize::conv(spans[first].2);
                let second_begin = usize::conv(spans[second].1);
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
                debug_assert!(spans[first].1 <= spans[second].1);
                spans[first].2 = spans[first].2.max(spans[second].2);

                spans.swap(second, len - 1);
                len -= 1;
                if j >= len {
                    i += 1;
                    j = i + 1;
                }
            }

            // We are left with non-overlapping spans.
            // For each span, we ensure cell widths are sufficiently large.
            for span in &spans[..len] {
                let rules = span.0;
                let begin = usize::conv(span.1);
                let end = usize::conv(span.2);
                rules.distribute_span_over(&mut widths[begin..end]);
            }

            widths.iter().sum()
        }

        let rules;
        if self.axis.is_horizontal() {
            rules = calculate(storage.width_rules(), self.col_spans.as_mut());
            storage.set_width_total(rules);
        } else {
            rules = calculate(storage.height_rules(), self.row_spans.as_mut());
            storage.set_height_total(rules);
        }
        rules
    }
}

/// A [`RulesSetter`] for grids supporting cell-spans
pub struct GridSetter<RT: RowTemp, CT: RowTemp, S: GridStorage> {
    w_offsets: RT,
    h_offsets: CT,
    pos: Coord,
    _s: PhantomData<S>,
}

impl<RT: RowTemp, CT: RowTemp, S: GridStorage> GridSetter<RT, CT, S> {
    /// Construct
    ///
    /// Argument order is consistent with other [`RulesSetter`]s.
    ///
    /// -   `rect`: the [`Rect`] within which to position children
    /// -   `(cols, rows)`: number of columns and rows
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
            let align = align.horiz.unwrap_or(Align::Default);
            let (widths, rules, total) = storage.widths_rules_total();
            let max_size = total.max_size();
            let mut target = rect.size.0;

            w_offsets.as_mut()[0] = 0;
            if target > max_size {
                let extra = target - max_size;
                target = max_size;
                w_offsets.as_mut()[0] = match align {
                    Align::Default | Align::TL | Align::Stretch => 0,
                    Align::Centre => extra / 2,
                    Align::BR => extra,
                };
            }

            SizeRules::solve_seq_total(widths, rules, total, target);
            for i in 1..w_offsets.as_mut().len() {
                let i1 = i - 1;
                let m1 = storage.width_rules()[i1].margins_i32().1;
                let m0 = storage.width_rules()[i].margins_i32().0;
                w_offsets.as_mut()[i] = w_offsets.as_mut()[i1] + storage.widths()[i1] + m1.max(m0);
            }
        }

        if rows > 0 {
            let align = align.vert.unwrap_or(Align::Default);
            let (heights, rules, total) = storage.heights_rules_total();
            let max_size = total.max_size();
            let mut target = rect.size.1;

            h_offsets.as_mut()[0] = 0;
            if target > max_size {
                let extra = target - max_size;
                target = max_size;
                h_offsets.as_mut()[0] = match align {
                    Align::Default | Align::TL | Align::Stretch => 0,
                    Align::Centre => extra / 2,
                    Align::BR => extra,
                };
            }

            SizeRules::solve_seq_total(heights, rules, total, target);
            for i in 1..h_offsets.as_mut().len() {
                let i1 = i - 1;
                let m1 = storage.height_rules()[i1].margins_i32().1;
                let m0 = storage.height_rules()[i].margins_i32().0;
                h_offsets.as_mut()[i] = h_offsets.as_mut()[i1] + storage.heights()[i1] + m1.max(m0);
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
        let x = self.w_offsets.as_mut()[usize::conv(info.col)];
        let y = self.h_offsets.as_mut()[usize::conv(info.row)];
        let pos = self.pos + Offset(x, y);

        let i1 = usize::conv(info.col_end) - 1;
        let w = storage.widths()[i1] + self.w_offsets.as_mut()[i1]
            - self.w_offsets.as_mut()[usize::conv(info.col)];
        let i1 = usize::conv(info.row_end) - 1;
        let h = storage.heights()[i1] + self.h_offsets.as_mut()[i1]
            - self.h_offsets.as_mut()[usize::conv(info.row)];
        let size = Size(w, h);

        Rect { pos, size }
    }

    fn maximal_rect_of(&mut self, _storage: &mut Self::Storage, _index: Self::ChildInfo) -> Rect {
        unimplemented!()
    }
}
