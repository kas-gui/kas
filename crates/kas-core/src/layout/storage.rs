// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver — storage

use super::{AxisInfo, SizeRules};
use crate::dir::Directional;
use crate::geom::{Offset, Rect, Size};
use crate::theme::{FrameStyle, SizeCx};
use kas_macros::impl_scope;
use std::fmt::Debug;

/// Layout storage for pack
#[derive(Clone, Default, Debug)]
pub struct PackStorage {
    /// Ideal size
    pub size: Size,
}

/// Layout storage for frame
#[derive(Clone, Default, Debug)]
pub struct FrameStorage {
    /// Size used by frame (sum of widths of borders)
    pub size: Size,
    /// Offset of frame contents from parent position
    pub offset: Offset,
    /// [`Rect`] assigned to whole frame
    ///
    /// NOTE: for a top-level layout component this is redundant with the
    /// widget's rect. For frames deeper within a widget's layout we *could*
    /// instead recalculate this (in every draw call etc.).
    pub rect: Rect,
}
impl FrameStorage {
    /// Calculate child's "other axis" size
    pub fn child_axis(&self, mut axis: AxisInfo) -> AxisInfo {
        axis.sub_other(self.size.extract(axis.flipped()));
        axis
    }

    /// Generate [`SizeRules`]
    pub fn size_rules(
        &mut self,
        cx: &mut SizeCx,
        axis: AxisInfo,
        child_rules: SizeRules,
        style: FrameStyle,
    ) -> SizeRules {
        let frame_rules = cx.frame(style, axis);
        let (rules, offset, size) = frame_rules.surround(child_rules);
        self.offset.set_component(axis, offset);
        self.size.set_component(axis, size);
        rules
    }
}

/// Requirements of row solver storage type
///
/// Usually this is set by a [`crate::layout::RowSolver`] from
/// [`crate::Layout::size_rules`], then used by [`crate::Layout::set_rect`] to
/// divide the assigned rect between children.
///
/// It may be useful to access this directly if not solving size rules normally;
/// specifically this allows a different size solver to replace `size_rules` and
/// influence `set_rect`.
///
/// Note: some implementations allocate when [`Self::set_dim`] is first called.
/// It is expected that this method is called before other methods.
pub trait RowStorage: Clone + Debug + sealed::Sealed {
    /// Set dimension: number of columns or rows
    fn set_dim(&mut self, cols: usize);

    /// Access [`SizeRules`] for each column/row
    fn rules(&mut self) -> &mut [SizeRules] {
        self.widths_and_rules().1
    }

    /// Access widths for each column/row
    ///
    /// Widths are calculated from rules when `set_rect` is called. Assigning
    /// to widths before `set_rect` is called only has any effect when the available
    /// size exceeds the minimum required (see [`SizeRules::solve_seq`]).
    fn widths(&mut self) -> &mut [i32] {
        self.widths_and_rules().0
    }

    /// Access widths and rules simultaneously
    fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]);
}

/// Fixed-length row storage
///
/// Uses const-generics argument `C` (the number of columns).
#[derive(Clone, Debug)]
pub struct FixedRowStorage<const C: usize> {
    rules: [SizeRules; C],
    widths: [i32; C],
}

impl<const C: usize> Default for FixedRowStorage<C> {
    fn default() -> Self {
        FixedRowStorage {
            rules: [SizeRules::default(); C],
            widths: [0; C],
        }
    }
}

impl<const C: usize> RowStorage for FixedRowStorage<C> {
    fn set_dim(&mut self, cols: usize) {
        assert_eq!(self.rules.as_ref().len(), cols);
        assert_eq!(self.widths.as_ref().len(), cols);
    }

    fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
        (self.widths.as_mut(), self.rules.as_mut())
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynRowStorage {
    rules: Vec<SizeRules>,
    widths: Vec<i32>,
}

impl RowStorage for DynRowStorage {
    fn set_dim(&mut self, cols: usize) {
        self.rules.resize(cols, SizeRules::EMPTY);
        self.widths.resize(cols, 0);
    }

    fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
        (&mut self.widths, &mut self.rules)
    }
}

/// Temporary storage type.
///
/// For dynamic-length rows and fixed-length rows with more than 16 items use
/// `Vec<i32>`. For fixed-length rows up to 16 items, use `[i32; rows]`.
pub trait RowTemp: AsMut<[i32]> + Default + Debug + sealed::Sealed {
    fn set_len(&mut self, len: usize);
}

impl RowTemp for Vec<i32> {
    fn set_len(&mut self, len: usize) {
        self.resize(len, 0);
    }
}

impl<const L: usize> RowTemp for [i32; L]
where
    [i32; L]: Default,
{
    fn set_len(&mut self, len: usize) {
        assert_eq!(self.len(), len);
    }
}

/// Requirements of grid solver storage type
///
/// Usually this is set by a [`crate::layout::GridSolver`] from
/// [`crate::Layout::size_rules`], then used by [`crate::Layout::set_rect`] to
/// divide the assigned rect between children.
///
/// It may be useful to access this directly if not solving size rules normally;
/// specifically this allows a different size solver to replace `size_rules` and
/// influence `set_rect`.
///
/// Note: some implementations allocate when [`Self::set_dims`] is first called.
/// It is expected that this method is called before other methods.
pub trait GridStorage: sealed::Sealed + Clone + std::fmt::Debug {
    /// Set dimension: number of columns and rows
    fn set_dims(&mut self, cols: usize, rows: usize);

    /// Access [`SizeRules`] for each column
    fn width_rules(&mut self) -> &mut [SizeRules] {
        self.widths_and_rules().1
    }

    /// Access [`SizeRules`] for each row
    fn height_rules(&mut self) -> &mut [SizeRules] {
        self.heights_and_rules().1
    }

    /// Access widths for each column
    ///
    /// Widths are calculated from rules when `set_rect` is called. Assigning
    /// to widths before `set_rect` is called only has any effect when the available
    /// size exceeds the minimum required (see [`SizeRules::solve_seq`]).
    fn widths(&mut self) -> &mut [i32] {
        self.widths_and_rules().0
    }

    /// Access heights for each row
    ///
    /// Heights are calculated from rules when `set_rect` is called. Assigning
    /// to heights before `set_rect` is called only has any effect when the available
    /// size exceeds the minimum required (see [`SizeRules::solve_seq`]).
    fn heights(&mut self) -> &mut [i32] {
        self.heights_and_rules().0
    }

    /// Access column widths and rules simultaneously
    fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]);

    /// Access row heights and rules simultaneously
    fn heights_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]);
}

impl_scope! {
    /// Fixed-length grid storage
    ///
    /// Uses const-generics arguments `R, C` (the number of rows and columns).
    #[impl_default]
    #[derive(Clone, Debug)]
    pub struct FixedGridStorage<const C: usize, const R: usize> {
        width_rules: [SizeRules; C] = [SizeRules::default(); C],
        height_rules: [SizeRules; R] = [SizeRules::default(); R],
        widths: [i32; C] = [0; C],
        heights: [i32; R] = [0; R],
    }

    impl GridStorage for Self {
        fn set_dims(&mut self, cols: usize, rows: usize) {
            assert_eq!(self.width_rules.as_ref().len(), cols);
            assert_eq!(self.height_rules.as_ref().len(), rows);
            assert_eq!(self.widths.len(), cols);
            assert_eq!(self.heights.len(), rows);
        }

        fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
            (
                self.widths.as_mut(),
                self.width_rules.as_mut(),
            )
        }
        fn heights_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
            (
                self.heights.as_mut(),
                self.height_rules.as_mut(),
            )
        }
    }
}

/// Variable-length grid storage
#[derive(Clone, Debug, Default)]
pub struct DynGridStorage {
    width_rules: Vec<SizeRules>,
    height_rules: Vec<SizeRules>,
    widths: Vec<i32>,
    heights: Vec<i32>,
}

impl GridStorage for DynGridStorage {
    fn set_dims(&mut self, cols: usize, rows: usize) {
        self.width_rules.resize(cols, SizeRules::EMPTY);
        self.height_rules.resize(rows, SizeRules::EMPTY);
        self.widths.resize(cols, 0);
        self.heights.resize(rows, 0);
    }

    fn widths_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
        (self.widths.as_mut(), self.width_rules.as_mut())
    }
    fn heights_and_rules(&mut self) -> (&mut [i32], &mut [SizeRules]) {
        (self.heights.as_mut(), self.height_rules.as_mut())
    }
}

mod sealed {
    pub trait Sealed {}
    impl<const C: usize> Sealed for super::FixedRowStorage<C> {}
    impl Sealed for super::DynRowStorage {}
    impl Sealed for Vec<i32> {}
    impl<const L: usize> Sealed for [i32; L] {}
    impl<const C: usize, const R: usize> Sealed for super::FixedGridStorage<C, R> {}
    impl Sealed for super::DynGridStorage {}
}
