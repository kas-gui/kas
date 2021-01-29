// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Safer type conversion utilities
//!
//! We define our own type conversion utilities since `From`, `TryFrom`,
//! `num_traits::NumCast` and `cast` (crate) don't always do what we need (or
//! can be cumbersome to use for fallible casts) and we prefer to avoid the `as`
//! keyword (which doesn't always preserve value).

use std::mem::size_of;

// Borrowed from static_assertions:
macro_rules! const_assert {
    ($x:expr $(,)?) => {
        #[allow(unknown_lints, eq_op)]
        const _: [(); 0 - !{
            const ASSERT: bool = $x;
            ASSERT
        } as usize] = [];
    };
}

const_assert!(size_of::<isize>() >= size_of::<i32>());
const_assert!(size_of::<isize>() <= size_of::<i64>());
const_assert!(size_of::<usize>() == size_of::<isize>());

/// Value conversion trait
///
/// Very roughly, this trait is [`From`] but with more assumptions (or
/// `T::try_from(x).unwrap()`), and restricted to numeric conversions.
///
/// -   Conversions should preserve values precisely
/// -   Conversions are expected to succeed but may fail
/// -   We assume that `isize` and `usize` are 32 or 64 bits
///
/// Fallible conversions are allowed. In Debug builds failure must always panic
/// but in Release builds this is not required (similar to overflow checks on
/// integer arithmetic).
///
/// [`From`]: std::convert::From
pub trait Conv<T> {
    fn conv(v: T) -> Self;
}

macro_rules! impl_via_from {
    ($x:ty: $y:ty) => {
        impl Conv<$x> for $y {
            #[inline]
            fn conv(x: $x) -> $y {
                <$y>::from(x)
            }
        }
    };
    ($x:ty: $y:ty, $($yy:ty),+) => {
        impl_via_from!($x: $y);
        impl_via_from!($x: $($yy),+);
    };
}

impl_via_from!(bool: i16, i32, i64, i128, isize);
impl_via_from!(bool: u8, u16, u32, u64, u128, usize);
impl_via_from!(f32: f64);
impl_via_from!(i8: f32, f64, i16, i32, i64, i128, isize);
impl_via_from!(i16: f32, f64, i32, i64, i128, isize);
impl_via_from!(i32: f64, i64, i128);
impl_via_from!(i64: i128);
impl_via_from!(u8: f32, f64, i16, i32, i64, i128, isize);
impl_via_from!(u8: u16, u32, u64, u128, usize);
impl_via_from!(u16: f32, f64, i32, i64, i128, u32, u64, u128, usize);
impl_via_from!(u32: f64, i64, i128, u32, u64, u128);
impl_via_from!(u64: i128, u128);

// These rely on the const assertions above
macro_rules! impl_via_as {
    ($x:ty: $y:ty) => {
        impl Conv<$x> for $y {
            #[inline]
            fn conv(x: $x) -> $y {
                x as $y
            }
        }
    };
    ($x:ty: $y:ty, $($yy:ty),+) => {
        impl_via_as!($x: $y);
        impl_via_as!($x: $($yy),+);
    };
}

impl_via_as!(i32: isize);
impl_via_as!(isize: i64, i128);
impl_via_as!(u32: usize);
impl_via_as!(usize: u64, u128);

impl Conv<u16> for isize {
    #[inline]
    fn conv(v: u16) -> isize {
        isize::conv(i32::from(v))
    }
}

macro_rules! impl_via_as_neg_check {
    ($x:ty: $y:ty) => {
        impl Conv<$x> for $y {
            #[inline]
            fn conv(x: $x) -> $y {
                debug_assert!(x >= 0);
                x as $y
            }
        }
    };
    ($x:ty: $y:ty, $($yy:ty),+) => {
        impl_via_as_neg_check!($x: $y);
        impl_via_as_neg_check!($x: $($yy),+);
    };
}

impl_via_as_neg_check!(i8: u8, u16, u32, u64, u128, usize);
impl_via_as_neg_check!(i16: u16, u32, u64, u128, usize);
impl_via_as_neg_check!(i32: u32, u64, u128, usize);
impl_via_as_neg_check!(i64: u64, u128);
impl_via_as_neg_check!(i128: u128);
impl_via_as_neg_check!(isize: u32, u64, u128, usize);

// Assumption: $y::MAX is representable as $x
macro_rules! impl_via_as_max_check {
    ($x:ty: $y:ty) => {
        impl Conv<$x> for $y {
            #[inline]
            fn conv(x: $x) -> $y {
                debug_assert!(x <= <$y>::MAX as $x);
                x as $y
            }
        }
    };
    ($x:ty: $y:ty, $($yy:ty),+) => {
        impl_via_as_max_check!($x: $y);
        impl_via_as_max_check!($x: $($yy),+);
    };
}

impl_via_as_max_check!(u16: i8, i16, u8);
impl_via_as_max_check!(u32: i8, i16, i32, u8, u16);
impl_via_as_max_check!(u64: i8, i16, i32, i64, u8, u16, u32, usize);
impl_via_as_max_check!(u128: i8, i16, i32, i64, i128, u8, u16, u32, u64, usize);
impl_via_as_max_check!(usize: i8, i16, i32, isize, u8, u16, u32);

// Assumption: $y::MAX and $y::MIN are representable as $x
macro_rules! impl_via_as_range_check {
    ($x:ty: $y:ty) => {
        impl Conv<$x> for $y {
            #[inline]
            fn conv(x: $x) -> $y {
                debug_assert!(<$y>::MIN as $x <= x && x <= <$y>::MAX as $x);
                x as $y
            }
        }
    };
    ($x:ty: $y:ty, $($yy:ty),+) => {
        impl_via_as_range_check!($x: $y);
        impl_via_as_range_check!($x: $($yy),+);
    };
}

impl_via_as_range_check!(i16: i8, u8);
impl_via_as_range_check!(i32: i8, i16, u8, u16);
impl_via_as_range_check!(i64: i8, i16, i32, isize, u8, u16, u32);
impl_via_as_range_check!(i128: i8, i16, i32, i64, isize, u8, u16, u32, u64);
impl_via_as_range_check!(isize: i8, i16, i32);
