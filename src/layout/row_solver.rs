// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, Direction, Margins, RulesSetter, RulesSolver, SizeRules, Storage};
use crate::geom::Rect;

#[derive(Clone, Debug, Default)]
pub struct FixedRowStorage<R: Clone> {
    rules: R,
}

impl<R: Clone> Storage for FixedRowStorage<R> {}

/// A [`RulesSolver`] for rows (and, without loss of generality, for columns).
///
/// This implementation relies on the caller to provide storage for solver data.
///
/// NOTE: ideally this would use const-generics, but those aren't stable (or
/// even usable) yet. This will likely be implemented in the future.
pub struct FixedRowSolver<D, R: Clone, T> {
    // Generalisation implies that axis.vert() is incorrect
    axis: AxisInfo,
    axis_is_vertical: bool,
    rules: SizeRules,
    widths: T,
    _d: PhantomData<D>,
    _r: PhantomData<R>,
}

impl<D: Direction, R, T> FixedRowSolver<D, R, T>
where
    R: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    T: Default + AsRef<[u32]> + AsMut<[u32]>,
{
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `storage`: reference to persistent storage
    pub fn new(axis: AxisInfo, storage: &mut FixedRowStorage<R>) -> Self {
        let mut widths = T::default();
        assert!(widths.as_ref().len() + 1 == storage.rules.as_ref().len());
        assert!(widths.as_ref().iter().all(|w| *w == 0));

        let axis_is_vertical = axis.vertical ^ D::is_vertical();

        if axis.has_fixed && axis_is_vertical {
            // TODO: cache this for use by set_rect?
            SizeRules::solve_seq(widths.as_mut(), storage.rules.as_ref(), axis.other_axis);
        }

        FixedRowSolver {
            axis,
            axis_is_vertical,
            rules: SizeRules::EMPTY,
            widths,
            _d: Default::default(),
            _r: Default::default(),
        }
    }
}

impl<D, R, T> RulesSolver for FixedRowSolver<D, R, T>
where
    R: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    T: AsRef<[u32]>,
{
    type Storage = FixedRowStorage<R>;
    type ChildInfo = usize;

    fn for_child<CR: FnOnce(AxisInfo) -> SizeRules>(
        &mut self,
        storage: &mut Self::Storage,
        child_info: Self::ChildInfo,
        child_rules: CR,
    ) {
        if self.axis.has_fixed && self.axis_is_vertical {
            self.axis.other_axis = self.widths.as_ref()[child_info];
        }
        let child_rules = child_rules(self.axis);
        if !self.axis_is_vertical {
            storage.rules.as_mut()[child_info] = child_rules;
            self.rules += child_rules;
        } else {
            self.rules = self.rules.max(child_rules);
        }
    }

    fn finish<ColIter, RowIter>(
        self,
        storage: &mut Self::Storage,
        _: ColIter,
        _: RowIter,
    ) -> SizeRules
    where
        ColIter: Iterator<Item = (usize, usize, usize)>,
        RowIter: Iterator<Item = (usize, usize, usize)>,
    {
        let cols = storage.rules.as_ref().len() - 1;
        if !self.axis_is_vertical {
            storage.rules.as_mut()[cols] = self.rules;
        }

        self.rules
    }
}

pub struct FixedRowSetter<D, R: Clone, T> {
    crect: Rect,
    inter: u32,
    widths: T,
    _d: PhantomData<D>,
    _r: PhantomData<R>,
}

impl<D: Direction, R, T> FixedRowSetter<D, R, T>
where
    R: Clone + AsRef<[SizeRules]>,
    T: Default + AsRef<[u32]> + AsMut<[u32]>,
{
    pub fn new(mut rect: Rect, margins: Margins, storage: &mut FixedRowStorage<R>) -> Self {
        let mut widths = T::default();
        assert!(widths.as_ref().len() + 1 == storage.rules.as_ref().len());

        rect.pos += margins.first;
        rect.size -= margins.first + margins.last;
        let mut crect = rect;

        let (width, inter) = if !D::is_vertical() {
            crect.size.0 = 0; // hack to get correct first offset
            (rect.size.0, margins.inter.0)
        } else {
            crect.size.1 = 0;
            (rect.size.1, margins.inter.1)
        };

        SizeRules::solve_seq(widths.as_mut(), storage.rules.as_ref(), width);

        FixedRowSetter {
            crect,
            inter,
            widths,
            _d: Default::default(),
            _r: Default::default(),
        }
    }
}

impl<D: Direction, R, T> RulesSetter for FixedRowSetter<D, R, T>
where
    R: Clone,
    T: AsRef<[u32]>,
{
    type Storage = FixedRowStorage<R>;
    type ChildInfo = usize;

    fn child_rect(&mut self, child_info: Self::ChildInfo) -> Rect {
        if !D::is_vertical() {
            self.crect.pos.0 += (self.crect.size.0 + self.inter) as i32;
            self.crect.size.0 = self.widths.as_ref()[child_info];
        } else {
            self.crect.pos.1 += (self.crect.size.1 + self.inter) as i32;
            self.crect.size.1 = self.widths.as_ref()[child_info];
        }
        self.crect
    }
}
