// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Alignment types

#[allow(unused)]
use super::Stretch; // for doc-links
use crate::geom::{Rect, Size};

pub use crate::text::Align;

/// Partial alignment information provided by the parent
///
/// *Hints* are optional. Widgets are expected to substitute default values
/// where hints are not provided.
///
/// The [`AlignHints::complete`] method is provided to conveniently apply
/// alignment to a widget within [`kas::Layout::set_rect`]:
/// ```
/// # use kas::layout::{Align, AlignHints};
/// # use kas::geom::*;
/// # let align = AlignHints::NONE;
/// # let rect = Rect::new(Coord::ZERO, Size::ZERO);
/// let pref_size = Size(30, 20); // usually size comes from SizeHandle
/// let rect = align
///     .complete(Align::Stretch, Align::Centre)
///     .aligned_rect(pref_size, rect);
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

    /// Complete via default alignments
    pub fn complete(&self, horiz: Align, vert: Align) -> CompleteAlignment {
        CompleteAlignment {
            halign: self.horiz.unwrap_or(horiz),
            valign: self.vert.unwrap_or(vert),
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
}

impl CompleteAlignment {
    /// Construct a rect of size `ideal` within `rect` using the given alignment
    ///
    /// Note: this does not stretch, even with [`Align::Stretch`], since widget
    /// stretching should be determined by the [`Stretch`] priority instead.
    pub fn aligned_rect(&self, ideal: Size, rect: Rect) -> Rect {
        let mut pos = rect.pos;
        let mut size = rect.size;
        if ideal.0 < size.0 {
            pos.0 += match self.halign {
                Align::Centre => (size.0 - ideal.0) / 2,
                Align::BR => size.0 - ideal.0,
                Align::Default | Align::TL | Align::Stretch => 0,
            };
            size.0 = ideal.0;
        }
        if ideal.1 < size.1 {
            pos.1 += match self.valign {
                Align::Centre => (size.1 - ideal.1) / 2,
                Align::BR => size.1 - ideal.1,
                Align::Default | Align::TL | Align::Stretch => 0,
            };
            size.1 = ideal.1;
        }
        Rect { pos, size }
    }
}
