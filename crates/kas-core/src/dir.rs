// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Direction types

use std::fmt;

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

    /// Direction reversed along axis (i.e. Left ↔ Right)
    ///
    /// This allows compile-time selection of the reversed direction.
    type Reversed: Directional;

    /// Flip over diagonal (i.e. Down ↔ Right)
    #[must_use = "method does not modify self but returns a new value"]
    fn flipped(self) -> Self::Flipped;

    /// Reverse along axis (i.e. Left ↔ Right)
    #[must_use = "method does not modify self but returns a new value"]
    fn reversed(self) -> Self::Reversed;

    /// Convert to the [`Direction`] enum
    #[must_use = "method does not modify self but returns a new value"]
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
    ($d:ident, $df:ident, $dr:ident) => {
        /// Zero-sized instantiation of [`Directional`]
        #[derive(Copy, Clone, Default, Debug)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $d;
        impl Directional for $d {
            type Flipped = $df;
            type Reversed = $dr;
            #[inline]
            fn flipped(self) -> Self::Flipped {
                $df
            }
            #[inline]
            fn reversed(self) -> Self::Reversed {
                $dr
            }
            #[inline]
            fn as_direction(self) -> Direction {
                Direction::$d
            }
        }
    };
}
fixed!(Left, Up, Right);
fixed!(Right, Down, Left);
fixed!(Up, Left, Down);
fixed!(Down, Right, Up);

/// Axis-aligned directions
///
/// This is a variable instantiation of [`Directional`].
///
/// A default direction is provided, though somewhat arbitrary: `Right`.
#[crate::impl_default(Direction::Right)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Direction {
    Right = 0,
    Down = 1,
    Left = 2,
    Up = 3,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", match self {
            Direction::Right => "Right",
            Direction::Down => "Down",
            Direction::Left => "Left",
            Direction::Up => "Up",
        })
    }
}

impl Directional for Direction {
    type Flipped = Self;
    type Reversed = Self;

    fn flipped(self) -> Self::Flipped {
        use Direction::*;
        match self {
            Right => Down,
            Down => Right,
            Left => Up,
            Up => Left,
        }
    }

    fn reversed(self) -> Self::Reversed {
        use Direction::*;
        match self {
            Right => Left,
            Down => Up,
            Left => Right,
            Up => Down,
        }
    }

    #[inline]
    fn as_direction(self) -> Direction {
        self
    }
}

bitflags! {
    /// Multi-direction selector
    pub struct Directions: u8 {
        const LEFT = 0b0001;
        const RIGHT = 0b0010;
        const UP = 0b0100;
        const DOWN = 0b1000;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn size() {
        assert_eq!(size_of::<Left>(), 0);
        assert_eq!(size_of::<Right>(), 0);
        assert_eq!(size_of::<Up>(), 0);
        assert_eq!(size_of::<Down>(), 0);
        assert_eq!(size_of::<Direction>(), 1);
    }
}
