// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Alignment types

#[allow(unused)] use super::Stretch; // for doc-links
use crate::dir::Directional;
use crate::geom::{Rect, Size};

pub use crate::text::Align;

/// Partial alignment information provided by the parent
///
/// *Hints* are optional. Widgets are expected to substitute default values
/// where hints are not provided.
///
/// The [`AlignHints::complete`] method is provided to conveniently apply
/// alignment to a widget within [`crate::Layout::set_rect`]:
/// ```
/// # use kas_core::layout::{Align, AlignHints};
/// # use kas_core::geom::*;
/// # let align = AlignHints::NONE;
/// # let rect = Rect::new(Coord::ZERO, Size::ZERO);
/// let pref_size = Size(30, 20); // usually size comes from SizeCx
/// let rect = align
///     .complete(Align::Stretch, Align::Center)
///     .aligned_rect(pref_size, rect);
/// // self.core.rect = rect;
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AlignHints {
    pub horiz: Option<Align>,
    pub vert: Option<Align>,
}

impl AlignHints {
    /// No hints
    pub const NONE: AlignHints = AlignHints::new(None, None);

    /// Top, no horizontal hint
    pub const TOP: AlignHints = AlignHints::new(None, Some(Align::TL));
    /// Bottom, no horizontal hint
    pub const BOTTOM: AlignHints = AlignHints::new(None, Some(Align::BR));
    /// Left, no vertical hint
    pub const LEFT: AlignHints = AlignHints::new(Some(Align::TL), None);
    /// Right, no vertical hint
    pub const RIGHT: AlignHints = AlignHints::new(Some(Align::BR), None);

    /// Top, left
    pub const TOP_LEFT: AlignHints = AlignHints::new(Some(Align::TL), Some(Align::TL));
    /// Top, right
    pub const TOP_RIGHT: AlignHints = AlignHints::new(Some(Align::TL), Some(Align::BR));
    /// Bottom, left
    pub const BOTTOM_LEFT: AlignHints = AlignHints::new(Some(Align::BR), Some(Align::TL));
    /// Bottom, right
    pub const BOTTOM_RIGHT: AlignHints = AlignHints::new(Some(Align::BR), Some(Align::BR));

    /// Center on both axes
    pub const CENTER: AlignHints = AlignHints::new(Some(Align::Center), Some(Align::Center));
    /// Top, center
    pub const TOP_CENTER: AlignHints = AlignHints::new(Some(Align::Center), Some(Align::TL));
    /// Bottom, center
    pub const BOTTOM_CENTER: AlignHints = AlignHints::new(Some(Align::Center), Some(Align::BR));
    /// Center, left
    pub const CENTER_LEFT: AlignHints = AlignHints::new(Some(Align::TL), Some(Align::Center));
    /// Center, right
    pub const CENTER_RIGHT: AlignHints = AlignHints::new(Some(Align::BR), Some(Align::Center));

    /// Stretch on both axes
    pub const STRETCH: AlignHints = AlignHints::new(Some(Align::Stretch), Some(Align::Stretch));

    /// Construct with optional horiz. and vert. alignment
    pub const fn new(horiz: Option<Align>, vert: Option<Align>) -> Self {
        Self { horiz, vert }
    }

    /// Take horizontal/vertical component
    #[inline]
    pub fn extract(self, dir: impl Directional) -> Option<Align> {
        match dir.is_vertical() {
            false => self.horiz,
            true => self.vert,
        }
    }

    /// Set one component of self, based on a direction
    #[inline]
    pub fn set_component<D: Directional>(&mut self, dir: D, align: Option<Align>) {
        match dir.is_vertical() {
            false => self.horiz = align,
            true => self.vert = align,
        }
    }

    /// Combine two hints (first takes priority)
    #[must_use = "method does not modify self but returns a new value"]
    pub fn combine(self, rhs: AlignHints) -> Self {
        Self {
            horiz: self.horiz.or(rhs.horiz),
            vert: self.vert.or(rhs.vert),
        }
    }

    /// Unwrap type's alignments or substitute parameters
    pub fn unwrap_or(self, horiz: Align, vert: Align) -> (Align, Align) {
        (self.horiz.unwrap_or(horiz), self.vert.unwrap_or(vert))
    }

    /// Complete via default alignments
    pub fn complete(&self, horiz: Align, vert: Align) -> AlignPair {
        AlignPair::new(self.horiz.unwrap_or(horiz), self.vert.unwrap_or(vert))
    }
}

/// Provides alignment information on both axes along with ideal size
///
/// Note that the `ideal` size detail is only used for non-stretch alignment.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AlignPair {
    pub horiz: Align,
    pub vert: Align,
}

impl AlignPair {
    /// Default on both axes
    pub const DEFAULT: AlignPair = AlignPair::new(Align::Default, Align::Default);

    /// Center on both axes
    pub const CENTER: AlignPair = AlignPair::new(Align::Center, Align::Center);

    /// Stretch on both axes
    pub const STRETCH: AlignPair = AlignPair::new(Align::Stretch, Align::Stretch);

    /// Construct with horiz. and vert. alignment
    pub const fn new(horiz: Align, vert: Align) -> Self {
        Self { horiz, vert }
    }

    /// Extract one component, based on a direction
    #[inline]
    pub fn extract<D: Directional>(self, dir: D) -> Align {
        match dir.is_vertical() {
            false => self.horiz,
            true => self.vert,
        }
    }

    /// Set one component of self, based on a direction
    #[inline]
    pub fn set_component<D: Directional>(&mut self, dir: D, align: Align) {
        match dir.is_vertical() {
            false => self.horiz = align,
            true => self.vert = align,
        }
    }

    /// Construct a rect of size `ideal` within `rect` using the given alignment
    pub fn aligned_rect(&self, ideal: Size, rect: Rect) -> Rect {
        let mut pos = rect.pos;
        let mut size = rect.size;
        if ideal.0 < size.0 && self.horiz != Align::Stretch {
            pos.0 += match self.horiz {
                Align::Center => (size.0 - ideal.0) / 2,
                Align::BR => size.0 - ideal.0,
                Align::Default | Align::TL | Align::Stretch => 0,
            };
            size.0 = ideal.0;
        }
        if ideal.1 < size.1 && self.vert != Align::Stretch {
            pos.1 += match self.vert {
                Align::Center => (size.1 - ideal.1) / 2,
                Align::BR => size.1 - ideal.1,
                Align::Default | Align::TL | Align::Stretch => 0,
            };
            size.1 = ideal.1;
        }
        Rect { pos, size }
    }
}

impl From<(Align, Align)> for AlignPair {
    #[inline]
    fn from(p: (Align, Align)) -> Self {
        AlignPair::new(p.0, p.1)
    }
}

impl From<AlignPair> for (Align, Align) {
    #[inline]
    fn from(p: AlignPair) -> Self {
        (p.horiz, p.vert)
    }
}
