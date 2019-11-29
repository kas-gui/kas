// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, Margins, RulesSetter, RulesSolver, SizeRules, Storage};
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

#[derive(Clone, Debug, Default)]
pub struct FixedGridStorage<WR: Clone, HR: Clone> {
    width_rules: WR,
    height_rules: HR,
}

impl<WR: Clone, HR: Clone> Storage for FixedGridStorage<WR, HR> {}

/// A [`RulesSolver`] for grids supporting cell-spans
///
/// This implementation relies on the caller to provide storage for solver data.
pub struct FixedGridSolver<WR: Clone, HR: Clone, W, H, CSR, RSR> {
    axis: AxisInfo,
    widths: W,
    heights: H,
    col_span_rules: CSR,
    row_span_rules: RSR,
    _wr: PhantomData<WR>,
    _hr: PhantomData<HR>,
}

impl<WR, HR, W, H, CSR, RSR> FixedGridSolver<WR, HR, W, H, CSR, RSR>
where
    WR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    W: Default + AsRef<[u32]> + AsMut<[u32]>,
    H: Default + AsRef<[u32]> + AsMut<[u32]>,
    CSR: Default,
    RSR: Default,
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, storage: &mut FixedGridStorage<WR, HR>) -> Self {
        let widths = W::default();
        let heights = H::default();
        let col_span_rules = CSR::default();
        let row_span_rules = RSR::default();

        assert!(widths.as_ref().len() + 1 == storage.width_rules.as_ref().len());
        assert!(heights.as_ref().len() + 1 == storage.height_rules.as_ref().len());
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        assert!(heights.as_ref().iter().all(|w| *w == 0));

        let mut solver = FixedGridSolver {
            axis,
            widths,
            heights,
            col_span_rules,
            row_span_rules,
            _wr: Default::default(),
            _hr: Default::default(),
        };
        solver.prepare(storage);
        solver
    }

    fn prepare(&mut self, storage: &mut FixedGridStorage<WR, HR>) {
        if self.axis.has_fixed {
            // TODO: cache this for use by set_rect?
            if self.axis.vertical {
                SizeRules::solve_seq(
                    self.widths.as_mut(),
                    storage.width_rules.as_ref(),
                    self.axis.other_axis,
                );
            } else {
                SizeRules::solve_seq(
                    self.heights.as_mut(),
                    storage.height_rules.as_ref(),
                    self.axis.other_axis,
                );
            }
        }

        if !self.axis.vertical {
            for n in 0..storage.width_rules.as_ref().len() {
                storage.width_rules.as_mut()[n] = SizeRules::EMPTY;
            }
        } else {
            for n in 0..storage.height_rules.as_ref().len() {
                storage.height_rules.as_mut()[n] = SizeRules::EMPTY;
            }
        }
    }
}

impl<WR, HR, W, H, CSR, RSR> RulesSolver for FixedGridSolver<WR, HR, W, H, CSR, RSR>
where
    WR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    W: AsRef<[u32]>,
    H: AsRef<[u32]>,
    CSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    RSR: AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    type Storage = FixedGridStorage<WR, HR>;
    type ChildInfo = GridChildInfo;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
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
        let child_rules = child_rules(self.axis);
        let rules = if !self.axis.vertical {
            if child_info.col_span_index == std::usize::MAX {
                &mut storage.width_rules.as_mut()[child_info.col]
            } else {
                &mut self.col_span_rules.as_mut()[child_info.col_span_index]
            }
        } else {
            if child_info.row_span_index == std::usize::MAX {
                &mut storage.height_rules.as_mut()[child_info.row]
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
        let cols = storage.width_rules.as_ref().len() - 1;
        let rows = storage.height_rules.as_ref().len() - 1;

        let rules;
        if !self.axis.vertical {
            for span in col_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| storage.width_rules.as_ref()[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                storage.width_rules.as_mut()[start]
                    .set_at_least_op_sub(self.col_span_rules.as_ref()[ind], sum);
            }

            rules = storage.width_rules.as_ref()[0..cols]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            storage.width_rules.as_mut()[cols] = rules;
        } else {
            for span in row_spans {
                let start = span.0 as usize;
                let end = span.1 as usize;
                let ind = span.2 as usize;

                let sum = (start..end)
                    .map(|n| storage.height_rules.as_ref()[n])
                    .fold(SizeRules::EMPTY, |x, y| x + y);
                storage.height_rules.as_mut()[start]
                    .set_at_least_op_sub(self.row_span_rules.as_ref()[ind], sum);
            }

            rules = storage.height_rules.as_ref()[0..rows]
                .iter()
                .copied()
                .fold(SizeRules::EMPTY, |rules, item| rules + item);
            storage.height_rules.as_mut()[rows] = rules;
        }

        rules
    }
}

pub struct FixedGridSetter<WR: Clone, HR: Clone, W, H> {
    widths: W,
    heights: H,
    col_pos: W,
    row_pos: H,
    pos: Coord,
    inter: Size,
    _wr: PhantomData<WR>,
    _hr: PhantomData<HR>,
}

impl<WR, HR, W, H> FixedGridSetter<WR, HR, W, H>
where
    WR: Clone + AsRef<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]>,
    W: Default + AsRef<[u32]> + AsMut<[u32]>,
    H: Default + AsRef<[u32]> + AsMut<[u32]>,
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `margins`: margin sizes
    /// - `storage`: reference to persistent storage
    pub fn new(mut rect: Rect, margins: Margins, storage: &mut FixedGridStorage<WR, HR>) -> Self {
        let mut widths = W::default();
        let mut heights = H::default();
        let cols = widths.as_ref().len();
        let rows = heights.as_ref().len();
        assert!(cols + 1 == storage.width_rules.as_ref().len());
        assert!(rows + 1 == storage.height_rules.as_ref().len());

        rect.pos += margins.first;
        rect.size -= margins.first + margins.last;
        let inter = margins.inter;

        SizeRules::solve_seq(widths.as_mut(), storage.width_rules.as_ref(), rect.size.0);
        SizeRules::solve_seq(heights.as_mut(), storage.height_rules.as_ref(), rect.size.1);

        let mut col_pos = W::default();
        let mut row_pos = H::default();
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

        FixedGridSetter {
            widths,
            heights,
            col_pos,
            row_pos,
            pos: rect.pos,
            inter,
            _wr: Default::default(),
            _hr: Default::default(),
        }
    }
}

impl<WR, HR, W, H> RulesSetter for FixedGridSetter<WR, HR, W, H>
where
    WR: Clone,
    HR: Clone,
    W: Default + AsRef<[u32]>,
    H: Default + AsRef<[u32]>,
{
    type Storage = FixedGridStorage<WR, HR>;
    type ChildInfo = GridChildInfo;

    fn child_rect(&mut self, child_info: Self::ChildInfo) -> Rect {
        let pos = self.pos
            + Coord(
                self.col_pos.as_ref()[child_info.col] as i32,
                self.row_pos.as_ref()[child_info.row] as i32,
            );

        let mut size = Size(
            self.inter.0 * (child_info.col_end - child_info.col - 1) as u32,
            self.inter.1 * (child_info.row_end - child_info.row - 1) as u32,
        );
        for n in child_info.col..child_info.col_end {
            size.0 += self.widths.as_ref()[n];
        }
        for n in child_info.row..child_info.row_end {
            size.1 += self.heights.as_ref()[n];
        }

        Rect { pos, size }
    }
}
