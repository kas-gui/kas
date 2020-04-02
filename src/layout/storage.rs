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
///
/// NOTE: ideally this would use const-generics, but those aren't stable (or
/// even usable) yet. This will likely be implemented in the future.
pub trait RowStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn set_dim(&mut self, cols: usize);

    #[doc(hidden)]
    fn rules(&mut self) -> &mut [SizeRules] {
        self.rules_and_widths().0
    }

    #[doc(hidden)]
    fn widths(&mut self) -> &mut [u32] {
        self.rules_and_widths().1
    }

    #[doc(hidden)]
    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]);
}

/// Fixed-length row storage
///
/// Argument types:
///
/// - `R` is expected to be `[SizeRules; cols + 1]`
/// - `W` is expected to be `[u32; cols]`
#[derive(Clone, Debug, Default)]
pub struct FixedRowStorage<R: Clone, W: Clone> {
    rules: R,
    widths: W,
}

impl<R: Clone, W: Clone> Storage for FixedRowStorage<R, W> {}

impl<R, W> RowStorage for FixedRowStorage<R, W>
where
    R: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    W: Clone + AsRef<[u32]> + AsMut<[u32]>,
{
    fn set_dim(&mut self, cols: usize) {
        assert_eq!(self.rules.as_ref().len(), cols + 1);
        assert_eq!(self.widths.as_ref().len(), cols);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (self.rules.as_mut(), self.widths.as_mut())
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynRowStorage {
    rules: Vec<SizeRules>,
    widths: Vec<u32>,
}

impl Storage for DynRowStorage {}

impl RowStorage for DynRowStorage {
    fn set_dim(&mut self, cols: usize) {
        self.rules.resize(cols + 1, SizeRules::EMPTY);
        self.widths.resize(cols, 0);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (&mut self.rules, &mut self.widths)
    }
}

/// Temporary storage type.
///
/// For dynamic-length rows and fixed-length rows with more than 16 items use
/// `Vec<u32>`. For fixed-length rows up to 16 items, use `[u32; rows]`.
pub trait RowTemp: Default + sealed::Sealed {
    #[doc(hidden)]
    fn as_mut(&mut self) -> &mut [u32];
    #[doc(hidden)]
    fn set_len(&mut self, len: usize);
}

impl RowTemp for Vec<u32> {
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
        impl RowTemp for [u32; $n] {
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

/// Requirements of grid solver storage type
///
/// Details are hidden (for internal use only).
///
/// NOTE: ideally this would use const-generics, but those aren't stable (or
/// even usable) yet. This will likely be implemented in the future.
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
    fn widths(&mut self) -> &mut [u32] {
        self.rules_and_widths().1
    }
    #[doc(hidden)]
    fn heights(&mut self) -> &mut [u32] {
        self.rules_and_heights().1
    }

    #[doc(hidden)]
    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]);
    #[doc(hidden)]
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [u32]);
}

#[derive(Clone, Debug, Default)]
pub struct FixedGridStorage<WR: Clone, HR: Clone, W: Clone, H: Clone> {
    width_rules: WR,
    height_rules: HR,
    widths: W,
    heights: H,
}

impl<WR: Clone, HR: Clone, W: Clone, H: Clone> Storage for FixedGridStorage<WR, HR, W, H> {}

impl<WR, HR, W, H> GridStorage for FixedGridStorage<WR, HR, W, H>
where
    WR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    W: Clone + AsRef<[u32]> + AsMut<[u32]>,
    H: Clone + AsRef<[u32]> + AsMut<[u32]>,
{
    fn set_dims(&mut self, cols: usize, rows: usize) {
        assert_eq!(self.width_rules.as_ref().len(), cols + 1);
        assert_eq!(self.height_rules.as_ref().len(), rows + 1);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (self.width_rules.as_mut(), self.widths.as_mut())
    }
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (self.height_rules.as_mut(), self.heights.as_mut())
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynGridStorage {
    width_rules: Vec<SizeRules>,
    height_rules: Vec<SizeRules>,
    widths: Vec<u32>,
    heights: Vec<u32>,
}

impl Storage for DynGridStorage {}

impl GridStorage for DynGridStorage {
    fn set_dims(&mut self, cols: usize, rows: usize) {
        self.width_rules.resize(cols + 1, SizeRules::EMPTY);
        self.height_rules.resize(rows + 1, SizeRules::EMPTY);
    }

    fn rules_and_widths(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (&mut self.width_rules, &mut self.widths)
    }
    fn rules_and_heights(&mut self) -> (&mut [SizeRules], &mut [u32]) {
        (&mut self.height_rules, &mut self.heights)
    }
}

mod sealed {
    pub trait Sealed {}
    impl<R: Clone, W: Clone> Sealed for super::FixedRowStorage<R, W> {}
    impl Sealed for super::DynRowStorage {}
    impl Sealed for Vec<u32> {}
    impl<WR: Clone, HR: Clone, W: Clone, H: Clone> Sealed for super::FixedGridStorage<WR, HR, W, H> {}
    impl Sealed for super::DynGridStorage {}
}
