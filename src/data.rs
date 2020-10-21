// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Data types

use std::cell::RefCell;
use std::convert::TryFrom;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::u32;

use super::Align;
use crate::geom::{Rect, Size};

// for doc use
#[allow(unused)]
use kas::event::UpdateHandle;

/// A future value, pending UI operation
///
/// This is a simpler form of future than the [`std::future::Future`] trait,
/// relying on the UI event system for execution. It also does not come with a
/// waker; since calling into widget code is only possible through the event
/// system, an [`UpdateHandle`] should be used to wake the recipient.
#[derive(Debug)]
pub struct Future<T: 'static>(Rc<RefCell<Option<T>>>);
impl<T: 'static> Future<T> {
    /// Construct, given a boxed closure to produce the final value
    ///
    /// Returns the future and a `finish` closure to set the value when done.
    pub fn new_box_fnmut<U: 'static>(
        mut f: Box<dyn FnMut(&mut U) -> T>,
    ) -> (Self, Box<dyn FnMut(&mut U)>) {
        let target: Rc<RefCell<Option<T>>> = Default::default();
        let t2 = target.clone();
        let finish: Box<dyn FnMut(&mut U)> = Box::new(move |u| *t2.borrow_mut() = Some(f(u)));
        (Future(target), finish)
    }

    /// Check whether this is finished
    pub fn is_finished(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// Returns a result on completion
    ///
    /// It may be worth checking [`Future::is_finished`] before calling this method.
    pub fn try_finish(self) -> Result<T, Self> {
        Rc::try_unwrap(self.0)
            .map(|cell| {
                cell.into_inner()
                    .unwrap_or_else(|| panic!("Future finished without setting a value!"))
            })
            .map_err(|target| Future(target))
    }
}

/// Widget identifier
///
/// All widgets are assigned an identifier which is unique within the window.
/// This type may be tested for equality and order.
///
/// This type is small and cheap to copy. Internally it is "NonZero", thus
/// `Option<WidgetId>` is a free extension (requires no extra memory).
///
/// Identifiers are assigned when configured and when re-configured
/// (via [`kas::TkAction::Reconfigure`]). Since user-code is not notified of a
/// re-configure, user-code should not store a `WidgetId`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetId(NonZeroU32);

impl WidgetId {
    pub(crate) const FIRST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(1) });
    const LAST: WidgetId = WidgetId(unsafe { NonZeroU32::new_unchecked(u32::MAX) });

    pub(crate) fn next(self) -> Self {
        WidgetId(NonZeroU32::new(self.0.get() + 1).unwrap())
    }
}

impl TryFrom<u32> for WidgetId {
    type Error = ();
    fn try_from(x: u32) -> Result<WidgetId, ()> {
        NonZeroU32::new(x).map(|n| WidgetId(n)).ok_or(())
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

#[test]
fn size_of_option_widget_id() {
    use std::mem::size_of;
    assert_eq!(size_of::<WidgetId>(), size_of::<Option<WidgetId>>());
}

/// Common widget data
///
/// All widgets should embed a `#[widget_core] core: CoreData` field.
#[derive(Clone, Default, Debug)]
pub struct CoreData {
    pub rect: Rect,
    pub id: WidgetId,
    pub disabled: bool,
}

/// Partial alignment information provided by the parent
///
/// *Hints* are optional. Widgets are expected to substitute default values
/// where hints are not provided.
///
/// The [`AlignHints::complete`] method is provided to conveniently apply
/// alignment to a widget within [`kas::Layout::set_rect`]:
/// ```
/// # use kas::{Align, AlignHints, geom::*};
/// # let align = AlignHints::NONE;
/// # let rect = Rect::new(Coord::ZERO, Size::ZERO);
/// let pref_size = Size(30, 20); // usually size comes from SizeHandle
/// let rect = align
///     .complete(Align::Stretch, Align::Centre, pref_size)
///     .apply(rect);
/// // self.core.rect = rect;
/// ```
#[derive(Copy, Clone, Debug, Default)]
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

    /// Unwrap type's alignments or substitute parameters
    pub fn unwrap_or(self, horiz: Align, vert: Align) -> (Align, Align) {
        (self.horiz.unwrap_or(horiz), self.vert.unwrap_or(vert))
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
#[derive(Copy, Clone, Debug)]
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
                Align::BR => size.0 - ideal.0,
                Align::Default | Align::TL | Align::Stretch => 0,
            } as i32;
            size.0 = ideal.0;
        }
        if self.valign != Align::Stretch && ideal.1 < size.1 {
            pos.1 += match self.valign {
                Align::Centre => (size.1 - ideal.1) / 2,
                Align::BR => size.1 - ideal.1,
                Align::Default | Align::TL | Align::Stretch => 0,
            } as i32;
            size.1 = ideal.1;
        }
        Rect { pos, size }
    }

    /// Get `Align` members
    pub fn align(&self) -> (Align, Align) {
        (self.halign, self.valign)
    }
}

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
        pub struct $d;
        impl Directional for $d {
            type Flipped = $df;
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
pub enum Direction {
    Right = 0,
    Down = 1,
    Left = 2,
    Up = 3,
}

impl Directional for Direction {
    type Flipped = Self;

    #[inline]
    fn as_direction(self) -> Direction {
        self
    }
}
