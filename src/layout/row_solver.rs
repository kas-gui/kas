// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Row / column solver

use std::marker::PhantomData;

use super::{AxisInfo, Direction, Margins, RulesSetter, RulesSolver, SizeRules, Storage};
use crate::geom::Rect;

/// Requirements of row solver storage type
///
/// Details are hidden (for internal use only).
///
/// NOTE: ideally this would use const-generics, but those aren't stable (or
/// even usable) yet. This will likely be implemented in the future.
pub trait RowStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn as_ref(&self) -> &[SizeRules];
    #[doc(hidden)]
    fn as_mut(&mut self) -> &mut [SizeRules];
    #[doc(hidden)]
    fn set_len(&mut self, len: usize);
}

/// Fixed-length row storage
///
/// Argument type is expected to be `[SizeRules; n]` where `n = rows + 1`.
#[derive(Clone, Debug, Default)]
pub struct FixedRowStorage<S: Clone> {
    rules: S,
}

impl<S: Clone> Storage for FixedRowStorage<S> {}

impl<S> RowStorage for FixedRowStorage<S>
where
    S: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    fn as_ref(&self) -> &[SizeRules] {
        self.rules.as_ref()
    }
    fn as_mut(&mut self) -> &mut [SizeRules] {
        self.rules.as_mut()
    }
    fn set_len(&mut self, len: usize) {
        assert_eq!(self.rules.as_ref().len(), len);
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynRowStorage {
    rules: Vec<SizeRules>,
}

impl Storage for DynRowStorage {}

impl RowStorage for DynRowStorage {
    fn as_ref(&self) -> &[SizeRules] {
        self.rules.as_ref()
    }
    fn as_mut(&mut self) -> &mut [SizeRules] {
        self.rules.as_mut()
    }
    fn set_len(&mut self, len: usize) {
        self.rules.resize(len, SizeRules::EMPTY);
    }
}

/// Temporary storage type.
///
/// For dynamic-length rows and fixed-length rows with more than 16 items use
/// `Vec<u32>`. For fixed-length rows up to 16 items, use `[u32; rows]`.
pub trait RowTemporary: Default + sealed::Sealed {
    #[doc(hidden)]
    fn as_ref(&self) -> &[u32];
    #[doc(hidden)]
    fn as_mut(&mut self) -> &mut [u32];
    #[doc(hidden)]
    fn set_len(&mut self, len: usize);
}

impl RowTemporary for Vec<u32> {
    fn as_ref(&self) -> &[u32] {
        self
    }
    fn as_mut(&mut self) -> &mut [u32] {
        self
    }
    fn set_len(&mut self, len: usize) {
        self.resize(len, 0);
    }
}

// TODO: use const generics
macro_rules! impl_row_temporary {
    ($n:literal) => {
        impl RowTemporary for [u32; $n] {
            fn as_ref(&self) -> &[u32] {
                self
            }
            fn as_mut(&mut self) -> &mut [u32] {
                self
            }
            fn set_len(&mut self, len: usize) {
                assert_eq!(self.len(), len);
            }
        }
        impl sealed::Sealed for [u32; $n] {}
    };
    ($n:literal $($more:literal)*) => {
        impl_row_temporary!($n);
        impl_row_temporary!($($more)*);
    };
}
impl_row_temporary!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16);

mod sealed {
    pub trait Sealed {}
    impl<S: Clone> Sealed for super::FixedRowStorage<S> {}
    impl Sealed for super::DynRowStorage {}
    impl Sealed for Vec<u32> {}
}

/// A [`RulesSolver`] for rows (and, without loss of generality, for columns).
///
/// This is parameterised over:
///
/// -   `T:` [`RowTemporary`] — temporary storage type
/// -   `S:` [`RowStorage`] — persistent storage type
pub struct RowSolver<T: RowTemporary, S: RowStorage> {
    // Generalisation implies that axis.vert() is incorrect
    axis: AxisInfo,
    axis_is_vertical: bool,
    rules: SizeRules,
    widths: T,
    _s: PhantomData<S>,
}

impl<T: RowTemporary, S: RowStorage> RowSolver<T, S> {
    /// Construct.
    ///
    /// - `axis`: `AxisInfo` instance passed into `size_rules`
    /// - `dim`: direction and number of items
    /// - `storage`: reference to persistent storage
    pub fn new<D: Direction>(axis: AxisInfo, dim: (D, usize), storage: &mut S) -> Self {
        let mut widths = T::default();
        widths.set_len(dim.1);
        assert!(widths.as_ref().iter().all(|w| *w == 0));
        storage.set_len(dim.1 + 1);

        let axis_is_vertical = axis.vertical ^ dim.0.is_vertical();

        if axis.has_fixed && axis_is_vertical {
            // TODO: cache this for use by set_rect?
            SizeRules::solve_seq(widths.as_mut(), storage.as_ref(), axis.other_axis);
        }

        RowSolver {
            axis,
            axis_is_vertical,
            rules: SizeRules::EMPTY,
            widths,
            _s: Default::default(),
        }
    }
}

impl<T: RowTemporary, S: RowStorage> RulesSolver for RowSolver<T, S> {
    type Storage = S;
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
            storage.as_mut()[child_info] = child_rules;
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
        let cols = storage.as_ref().len() - 1;
        if !self.axis_is_vertical {
            storage.as_mut()[cols] = self.rules;
        }

        self.rules
    }
}

/// A [`RulesSetter`] for rows (and, without loss of generality, for columns).
///
/// This is parameterised over:
///
/// -   `D:` [`Direction`] — whether this represents a row or a column
/// -   `T:` [`RowTemporary`] — temporary storage type
/// -   `S:` [`RowStorage`] — persistent storage type
pub struct RowSetter<D, T: RowTemporary, S: RowStorage> {
    crect: Rect,
    inter: u32,
    widths: T,
    direction: D,
    _s: PhantomData<S>,
}

impl<D: Direction, T: RowTemporary, S: RowStorage> RowSetter<D, T, S> {
    pub fn new(mut rect: Rect, margins: Margins, dim: (D, usize), storage: &mut S) -> Self {
        let mut widths = T::default();
        widths.set_len(dim.1);
        storage.set_len(dim.1 + 1);

        rect.pos += margins.first;
        rect.size -= margins.first + margins.last;
        let mut crect = rect;

        let (width, inter) = if dim.0.is_horizontal() {
            crect.size.0 = 0; // hack to get correct first offset
            (rect.size.0, margins.inter.0)
        } else {
            crect.size.1 = 0;
            (rect.size.1, margins.inter.1)
        };

        SizeRules::solve_seq(widths.as_mut(), storage.as_ref(), width);

        RowSetter {
            crect,
            inter,
            widths,
            direction: dim.0,
            _s: Default::default(),
        }
    }
}

impl<D: Direction, T: RowTemporary, S: RowStorage> RulesSetter for RowSetter<D, T, S> {
    type Storage = S;
    type ChildInfo = usize;

    fn child_rect(&mut self, child_info: Self::ChildInfo) -> Rect {
        if self.direction.is_horizontal() {
            self.crect.pos.0 += (self.crect.size.0 + self.inter) as i32;
            self.crect.size.0 = self.widths.as_ref()[child_info];
        } else {
            self.crect.pos.1 += (self.crect.size.1 + self.inter) as i32;
            self.crect.size.1 = self.widths.as_ref()[child_info];
        }
        self.crect
    }
}
