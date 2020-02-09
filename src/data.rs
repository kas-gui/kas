// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::convert::TryFrom;
use std::fmt;
use std::num::NonZeroU32;
use std::u32;

use crate::geom::{Rect, Size};

/// Widget identifier
///
/// All widgets within a window are assigned a unique numeric identifier. This
/// type may be tested for equality and order.
///
/// Note: identifiers are first assigned when a window is instantiated by the
/// toolkit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    pub(crate) const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl TryFrom<u64> for WidgetId {
    type Error = ();
    fn try_from(x: u64) -> Result<WidgetId, ()> {
        if x <= u32::MAX as u64 {
            if let Some(nz) = NonZeroU32::new(x as u32) {
                return Ok(WidgetId(nz));
            }
        }
        Err(())
    }
}

impl From<WidgetId> for u32 {
    #[inline]
    fn from(id: WidgetId) -> u32 {
        id.0.get()
    }
}

impl From<WidgetId> for u64 {
    #[inline]
    fn from(id: WidgetId) -> u64 {
        id.0.get() as u64
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        WidgetId::LAST
    }
}

impl fmt::Display for WidgetId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "#{}", self.0)
    }
}

/// Common widget data
///
/// All widgets should embed a `#[core] core: CoreData` field.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
}

/// Alignment of contents
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Align {
    /// Align to top or left (for left-to-right text)
    Begin,
    /// Align to centre
    Centre,
    /// Align to bottom or right (for left-to-right text)
    End,
    /// Attempt to align to both margins
    ///
    /// For text, this is known as "justified alignment".
    Stretch,
}

/// Default alignment: Stretch
impl Default for Align {
    fn default() -> Self {
        Align::Stretch
    }
}

/// Partial alignment information provided by the parent
#[derive(Debug, Default)]
pub struct AlignHints {
    pub horiz: Option<Align>,
    pub vert: Option<Align>,
}

impl AlignHints {
    /// No hints
    pub const NONE: AlignHints = AlignHints::new(None, None);

    /// Construct with optional horiz. and vert. alignment
    pub const fn new(horiz: Option<Align>, vert: Option<Align>) -> Self {
        Self { horiz, vert }
    }

    /// Complete via defaults and ideal size information
    pub fn complete(&self, horiz: Align, vert: Align, ideal: Size) -> CompleteAlignment {
        CompleteAlignment {
            halign: self.horiz.unwrap_or(horiz),
            valign: self.vert.unwrap_or(vert),
            ideal,
        }
    }
}

/// Provides alignment information on both axes along with ideal size
///
/// Note that the `ideal` size detail is only used for non-stretch alignment.
pub struct CompleteAlignment {
    halign: Align,
    valign: Align,
    ideal: Size,
}

impl CompleteAlignment {
    /// Adjust the given `rect` according to alignment, returning the result
    pub fn apply(&self, rect: Rect) -> Rect {
        let ideal = self.ideal;
        let mut pos = rect.pos;
        let mut size = rect.size;
        if self.halign != Align::Stretch && ideal.0 < size.0 {
            pos.0 += match self.halign {
                Align::Centre => (size.0 - ideal.0) / 2,
                Align::End => size.0 - ideal.0,
                Align::Begin | Align::Stretch => 0,
            } as i32;
            size.0 = ideal.0;
        }
        if self.valign != Align::Stretch && ideal.1 < size.1 {
            pos.1 += match self.valign {
                Align::Centre => (size.1 - ideal.1) / 2,
                Align::End => size.1 - ideal.1,
                Align::Begin | Align::Stretch => 0,
            } as i32;
            size.1 = ideal.1;
        }
        Rect { pos, size }
    }
}

/// Trait over directional types
///
/// Using a generic `<D: Directional>` over [`Direction`] allows compile-time
/// substitution via the [`Horizontal`] and [`Vertical`] instantiations.
pub trait Directional: Copy + Sized + std::fmt::Debug {
    fn as_direction(self) -> Direction;

    #[inline]
    fn is_vertical(self) -> bool {
        self.as_direction() == Direction::Vertical
    }

    #[inline]
    fn is_horizontal(self) -> bool {
        self.as_direction() == Direction::Horizontal
    }
}

/// Fixed instantiation of [`Directional`]
#[derive(Copy, Clone, Default, Debug)]
pub struct Horizontal;
impl Directional for Horizontal {
    #[inline]
    fn as_direction(self) -> Direction {
        Direction::Horizontal
    }
}

/// Fixed instantiation of [`Directional`]
#[derive(Copy, Clone, Default, Debug)]
pub struct Vertical;
impl Directional for Vertical {
    #[inline]
    fn as_direction(self) -> Direction {
        Direction::Vertical
    }
}

/// Horizontal / vertical direction
///
/// This is a variable instantiation of [`Directional`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Direction {
    Horizontal = 0,
    Vertical = 1,
}

impl Directional for Direction {
    #[inline]
    fn as_direction(self) -> Direction {
        self
    }
}
