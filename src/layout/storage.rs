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
pub trait RowTemp: Default + sealed::Sealed {
    #[doc(hidden)]
    fn as_ref(&self) -> &[u32];
    #[doc(hidden)]
    fn as_mut(&mut self) -> &mut [u32];
    #[doc(hidden)]
    fn set_len(&mut self, len: usize);
}

impl RowTemp for Vec<u32> {
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
        impl RowTemp for [u32; $n] {
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

/// Requirements of grid solver storage type
///
/// Details are hidden (for internal use only).
///
/// NOTE: ideally this would use const-generics, but those aren't stable (or
/// even usable) yet. This will likely be implemented in the future.
pub trait GridStorage: sealed::Sealed + Clone {
    #[doc(hidden)]
    fn width_ref(&self) -> &[SizeRules];
    #[doc(hidden)]
    fn width_mut(&mut self) -> &mut [SizeRules];
    #[doc(hidden)]
    fn set_width_len(&mut self, len: usize);
    #[doc(hidden)]
    fn height_ref(&self) -> &[SizeRules];
    #[doc(hidden)]
    fn height_mut(&mut self) -> &mut [SizeRules];
    #[doc(hidden)]
    fn set_height_len(&mut self, len: usize);
}

#[derive(Clone, Debug, Default)]
pub struct FixedGridStorage<WR: Clone, HR: Clone> {
    width_rules: WR,
    height_rules: HR,
}

impl<WR: Clone, HR: Clone> Storage for FixedGridStorage<WR, HR> {}

impl<WR, HR> GridStorage for FixedGridStorage<WR, HR>
where
    WR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
    HR: Clone + AsRef<[SizeRules]> + AsMut<[SizeRules]>,
{
    fn width_ref(&self) -> &[SizeRules] {
        self.width_rules.as_ref()
    }
    fn width_mut(&mut self) -> &mut [SizeRules] {
        self.width_rules.as_mut()
    }
    fn set_width_len(&mut self, len: usize) {
        assert_eq!(self.width_rules.as_ref().len(), len);
    }
    fn height_ref(&self) -> &[SizeRules] {
        self.height_rules.as_ref()
    }
    fn height_mut(&mut self) -> &mut [SizeRules] {
        self.height_rules.as_mut()
    }
    fn set_height_len(&mut self, len: usize) {
        assert_eq!(self.height_rules.as_ref().len(), len);
    }
}

/// Variable-length row storage
#[derive(Clone, Debug, Default)]
pub struct DynGridStorage {
    width_rules: Vec<SizeRules>,
    height_rules: Vec<SizeRules>,
}

impl Storage for DynGridStorage {}

impl GridStorage for DynGridStorage {
    fn width_ref(&self) -> &[SizeRules] {
        self.width_rules.as_ref()
    }
    fn width_mut(&mut self) -> &mut [SizeRules] {
        self.width_rules.as_mut()
    }
    fn set_width_len(&mut self, len: usize) {
        self.width_rules.resize(len, SizeRules::EMPTY);
    }
    fn height_ref(&self) -> &[SizeRules] {
        self.height_rules.as_ref()
    }
    fn height_mut(&mut self) -> &mut [SizeRules] {
        self.height_rules.as_mut()
    }
    fn set_height_len(&mut self, len: usize) {
        self.height_rules.resize(len, SizeRules::EMPTY);
    }
}

mod sealed {
    pub trait Sealed {}
    impl<S: Clone> Sealed for super::FixedRowStorage<S> {}
    impl Sealed for super::DynRowStorage {}
    impl Sealed for Vec<u32> {}
    impl<WR: Clone, HR: Clone> Sealed for super::FixedGridStorage<WR, HR> {}
    impl Sealed for super::DynGridStorage {}
}
