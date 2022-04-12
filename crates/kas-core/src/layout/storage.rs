// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Layout solver — storage

use super::SizeRules;
use kas_macros::impl_scope;
use std::any::Any;

/// Master trait over storage types
pub trait Storage: Any + std::fmt::Debug {
    /// Get self as type `Any` (mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn Storage {
    /// Forwards to the method defined on the type `Any`.
    #[inline]
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        <dyn Any>::downcast_mut::<T>(self.as_any_mut())
    }
}

/// Empty storage type
impl Storage for () {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

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

impl<const C: usize> Storage for FixedRowStorage<C> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

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

impl Storage for DynRowStorage {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

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
pub trait RowTemp: AsMut<[i32]> + Default + sealed::Sealed {
    #[doc(hidden)]
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
/// Details are hidden (for internal use only).
pub trait GridStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn set_dims(&mut self, cols: usize, rows: usize);

    #[doc(hidden)]
    fn width_rules(&mut self) -> &mut [SizeRules] {
        self.widths_rules_total().1
    }
    #[doc(hidden)]
    fn height_rules(&mut self) -> &mut [SizeRules] {
        self.heights_rules_total().1
    }

    #[doc(hidden)]
    fn set_width_total(&mut self, total: SizeRules);
    #[doc(hidden)]
    fn set_height_total(&mut self, total: SizeRules);

    #[doc(hidden)]
    fn widths(&mut self) -> &mut [i32] {
        self.widths_rules_total().0
    }
    #[doc(hidden)]
    fn heights(&mut self) -> &mut [i32] {
        self.heights_rules_total().0
    }

    #[doc(hidden)]
    fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules);
    #[doc(hidden)]
    fn heights_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules);
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
        width_total: SizeRules,
        height_total: SizeRules,
        widths: [i32; C] = [0; C],
        heights: [i32; R] = [0; R],
    }

    impl Storage for Self {
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    impl GridStorage for Self {
        fn set_dims(&mut self, cols: usize, rows: usize) {
            assert_eq!(self.width_rules.as_ref().len(), cols);
            assert_eq!(self.height_rules.as_ref().len(), rows);
            assert_eq!(self.widths.len(), cols);
            assert_eq!(self.heights.len(), rows);
        }

        #[doc(hidden)]
        fn set_width_total(&mut self, total: SizeRules) {
            self.width_total = total;
        }
        #[doc(hidden)]
        fn set_height_total(&mut self, total: SizeRules) {
            self.height_total = total;
        }

        #[doc(hidden)]
        fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
            (
                self.widths.as_mut(),
                self.width_rules.as_mut(),
                self.width_total,
            )
        }
        #[doc(hidden)]
        fn heights_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
            (
                self.heights.as_mut(),
                self.height_rules.as_mut(),
                self.height_total,
            )
        }
    }
}

/// Variable-length grid storage
#[derive(Clone, Debug, Default)]
pub struct DynGridStorage {
    width_rules: Vec<SizeRules>,
    height_rules: Vec<SizeRules>,
    width_total: SizeRules,
    height_total: SizeRules,
    widths: Vec<i32>,
    heights: Vec<i32>,
}

impl Storage for DynGridStorage {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl GridStorage for DynGridStorage {
    fn set_dims(&mut self, cols: usize, rows: usize) {
        self.width_rules.resize(cols, SizeRules::EMPTY);
        self.height_rules.resize(rows, SizeRules::EMPTY);
        self.widths.resize(cols, 0);
        self.heights.resize(rows, 0);
    }

    #[doc(hidden)]
    fn set_width_total(&mut self, total: SizeRules) {
        self.width_total = total;
    }
    #[doc(hidden)]
    fn set_height_total(&mut self, total: SizeRules) {
        self.height_total = total;
    }

    #[doc(hidden)]
    fn widths_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
        (
            self.widths.as_mut(),
            self.width_rules.as_mut(),
            self.width_total,
        )
    }
    #[doc(hidden)]
    fn heights_rules_total(&mut self) -> (&mut [i32], &mut [SizeRules], SizeRules) {
        (
            self.heights.as_mut(),
            self.height_rules.as_mut(),
            self.height_total,
        )
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
