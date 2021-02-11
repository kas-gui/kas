// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Direction types

/// Trait over directional types
///
/// This trait has a variable implementation, [`Direction`], and several fixed
/// implementations, [`Right`], [`Down`], [`Left`] and [`Up`].
///
/// Using a generic `<D: Directional>` allows compile-time substitution of
/// direction information when parametrised with fixed implementations.
pub trait Directional: Copy + Sized + std::fmt::Debug + 'static {
    /// Direction flipped over diagonal (i.e. Down ↔ Right)
    ///
    /// This allows compile-time selection of the flipped direction.
    type Flipped: Directional;

    /// Flip over diagonal (i.e. Down ↔ Right)
    fn flipped(self) -> Self::Flipped;

    /// Convert to the [`Direction`] enum
    fn as_direction(self) -> Direction;

    /// Up or Down
    #[inline]
    fn is_vertical(self) -> bool {
        ((self.as_direction() as u32) & 1) == 1
    }

    /// Left or Right
    #[inline]
    fn is_horizontal(self) -> bool {
        ((self.as_direction() as u32) & 1) == 0
    }

    /// Left or Up
    #[inline]
    fn is_reversed(self) -> bool {
        ((self.as_direction() as u32) & 2) == 2
    }
}

macro_rules! fixed {
    [] => {};
    [($d:ident, $df:ident)] => {
        /// Fixed instantiation of [`Directional`]
        #[derive(Copy, Clone, Default, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $d;
        impl Directional for $d {
            type Flipped = $df;
            #[inline]
            fn flipped(self) -> Self::Flipped {
                $df
            }
            #[inline]
            fn as_direction(self) -> Direction {
                Direction::$d
            }
        }
    };
    [($d:ident, $df:ident), $(($d1:ident, $d2:ident),)*] => {
        fixed![($d, $df)];
        fixed![($df, $d)];
        fixed![$(($d1, $d2),)*];
    };
}
fixed![(Right, Down), (Left, Up),];

/// Axis-aligned directions
///
/// This is a variable instantiation of [`Directional`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    Right = 0,
    Down = 1,
    Left = 2,
    Up = 3,
}

impl Directional for Direction {
    type Flipped = Self;

    fn flipped(self) -> Self::Flipped {
        use Direction::*;
        match self {
            Right => Down,
            Down => Right,
            Left => Up,
            Up => Left,
        }
    }

    #[inline]
    fn as_direction(self) -> Direction {
        self
    }
}
