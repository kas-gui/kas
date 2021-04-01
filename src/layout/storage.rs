// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver — storage

use super::SizeRules;

/// Master trait over storage types
pub trait Storage {}

/// Requirements of row solver storage type
///
/// Details are hidden (for internal use only).
pub trait RowStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn set_dim(&mut self, cols: usize);

    #[doc(hidden)]
    fn rules(&mut self) -> &mut [SizeRules] {
        self.widths_rules_total().1
    }

    #[doc(hidden)]
    fn set_total(&mut self, total: SizeRules);

    #[doc(hidden)]
    fn widths(&mut self) -> &mut [i32] {
        self.widths_rules_total().0
    }

    #[doc(hidden)]
    fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules);
}

/// Fixed-length row storage
///
/// Uses const-generics argument `C` (the number of columns).
#[derive(Clone, Debug)]
pub struct FixedRowStorage<const C: usize> {
    rules: [SizeRules; C],
    total: SizeRules,
    widths: [i32; C],
}

impl<const C: usize> Default for FixedRowStorage<C> {
    fn default() -> Self {
        FixedRowStorage {
            rules: [SizeRules::default(); C],
            total: SizeRules::default(),
            widths: [0; C],
        }
    }
}

impl<const C: usize> Storage for FixedRowStorage<C> {}

impl<const C: usize> RowStorage for FixedRowStorage<C> {
    fn set_dim(&mut self, cols: usize) {
        assert_eq!(self.rules.as_ref().len(), cols);
        assert_eq!(self.widths.as_ref().len(), cols);
    }

    fn set_total(&mut self, total: SizeRules) {
        self.total = total;
    }

    fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
        (self.widths.as_mut(), self.rules.as_mut(), self.total)
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynRowStorage {
    rules: Vec<SizeRules>,
    total: SizeRules,
    widths: Vec<i32>,
}

impl Storage for DynRowStorage {}

impl RowStorage for DynRowStorage {
    fn set_dim(&mut self, cols: usize) {
        self.rules.resize(cols, SizeRules::EMPTY);
        self.widths.resize(cols, 0);
    }

    fn set_total(&mut self, total: SizeRules) {
        self.total = total;
    }

    fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
        (&mut self.widths, &mut self.rules, self.total)
    }
}

/// Temporary storage type.
///
/// For dynamic-length rows and fixed-length rows with more than 16 items use
/// `Vec<i32>`. For fixed-length rows up to 16 items, use `[i32; rows]`.
pub trait RowTemp: Default + sealed::Sealed {
    #[doc(hidden)]
    fn as_mut(&mut self) -> &mut [i32];
    #[doc(hidden)]
    fn set_len(&mut self, len: usize);
}

impl RowTemp for Vec<i32> {
    fn as_mut(&mut self) -> &mut [i32] {
        self
    }
    fn set_len(&mut self, len: usize) {
        self.resize(len, 0);
    }
}

// TODO: use const generics
macro_rules! impl_row_temporary {
    ($n:literal) => {
        impl RowTemp for [i32; $n] {
            fn as_mut(&mut self) -> &mut [i32] {
                self
            }
            fn set_len(&mut self, len: usize) {
                assert_eq!(self.len(), len);
            }
        }
        impl sealed::Sealed for [i32; $n] {}
    };
    ($n:literal $($more:literal)*) => {
        impl_row_temporary!($n);
        impl_row_temporary!($($more)*);
    };
}
impl_row_temporary!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16);

/// Requirements of grid solver storage type
///
/// Details are hidden (for internal use only).
pub trait GridStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn set_dims(&mut self, cols: usize, rows: usize);

    #[doc(hidden)]
    fn width_rules(&mut self) -> &mut [SizeRules] {
        self.rules_and_widths().0
    }
    #[doc(hidden)]
    fn height_rules(&mut self) -> &mut [SizeRules] {
        self.rules_and_heights().0
    }

    #[doc(hidden)]
    fn widths(&mut self) -> &mut [i32] {
        self.rules_and_widths().1
    }
    #[doc(hidden)]
    fn heights(&mut self) -> &mut [i32] {
        self.rules_and_heights().1
    }

    #[doc(hidden)]
    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [i32]);
    #[doc(hidden)]
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [i32]);
}

/// Fixed-length grid storage
///
/// Uses const-generics arguments `C, R` (the number of columns and rows).
///
/// Argument types:
///
/// - `WR` is expected to be `[SizeRules; cols + 1]`
/// - `HR` is expected to be `[SizeRules; rows + 1]`
#[derive(Clone, Debug)]
pub struct FixedGridStorage<WR: Clone, HR: Clone, const C: usize, const R: usize> {
    width_rules: WR,
    height_rules: HR,
    widths: [i32; C],
    heights: [i32; R],
}

impl<WR: Clone + Default, HR: Clone + Default, const C: usize, const R: usize> Default
    for FixedGridStorage<WR, HR, C, R>
{
    fn default() -> Self {
        FixedGridStorage {
            width_rules: Default::default(),
            height_rules: Default::default(),
            widths: [0; C],
            heights: [0; R],
        }
    }
}

impl<WR: Clone, HR: Clone, const C: usize, const R: usize> Storage
    for FixedGridStorage<WR, HR, C, R>
{
}

impl<WR, HR, const C: usize, const R: usize> GridStorage for FixedGridStorage<WR, HR, C, R>
where
    WR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    fn set_dims(&mut self, cols: usize, rows: usize) {
        assert_eq!(self.width_rules.as_ref().len(), cols + 1);
        assert_eq!(self.height_rules.as_ref().len(), rows + 1);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [i32]) {
        (self.width_rules.as_mut(), self.widths.as_mut())
    }
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [i32]) {
        (self.height_rules.as_mut(), self.heights.as_mut())
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

impl Storage for DynGridStorage {}

impl GridStorage for DynGridStorage {
    fn set_dims(&mut self, cols: usize, rows: usize) {
        self.width_rules.resize(cols + 1, SizeRules::EMPTY);
        self.height_rules.resize(rows + 1, SizeRules::EMPTY);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [i32]) {
        (&mut self.width_rules, &mut self.widths)
    }
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [i32]) {
        (&mut self.height_rules, &mut self.heights)
    }
}

mod sealed {
    pub trait Sealed {}
    impl<const C: usize> Sealed for super::FixedRowStorage<C> {}
    impl Sealed for super::DynRowStorage {}
    impl Sealed for Vec<i32> {}
    impl<WR: Clone, HR: Clone, const C: usize, const R: usize> Sealed
        for super::FixedGridStorage<WR, HR, C, R>
    {
    }
    impl Sealed for super::DynGridStorage {}
}
