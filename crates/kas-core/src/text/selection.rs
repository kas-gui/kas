// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Tools for text selection

use std::ops::Range;

/// Cursor index and selection range
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorRange<Index> {
    /// The start or end of the selection.
    pub anchor: Index,
    /// The cursor (edit) index.
    pub cursor: Index,
}

impl<Index: Copy> From<Index> for CursorRange<Index> {
    #[inline]
    fn from(index: Index) -> Self {
        CursorRange {
            anchor: index,
            cursor: index,
        }
    }
}

impl<Index> From<Range<Index>> for CursorRange<Index> {
    #[inline]
    fn from(range: Range<Index>) -> Self {
        CursorRange {
            anchor: range.start,
            cursor: range.end,
        }
    }
}

impl<Index: Copy + Eq + Ord> CursorRange<Index> {
    /// True if the selection index equals the cursor index
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cursor == self.anchor
    }

    /// Convert to a [`Range`], increasing
    ///
    /// The return value has `range.start <= range.end`.
    pub fn to_range(&self) -> Range<Index> {
        let mut range = *self;
        if range.anchor > range.cursor {
            range.reverse();
        }
        range.anchor..range.cursor
    }

    /// Reverse the selection
    ///
    /// Swaps the selection and edit indices. The result of [`Self::to_range`] is
    /// not affected by this method.
    #[inline]
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.anchor, &mut self.cursor);
    }

    /// Clear selection
    ///
    /// Sets the selection index to the edit index.
    #[inline]
    pub fn clear_selection(&mut self) {
        self.anchor = self.cursor;
    }

    /// Set the cursor position and clear the selection
    ///
    /// Both indices are set to `index`.
    #[inline]
    pub fn set_position(&mut self, index: Index) {
        self.anchor = index;
        self.cursor = index;
    }
}

impl CursorRange<usize> {
    /// Fix selection for updated text
    ///
    /// This method ensures that indices lie on char boundaries and are not
    /// beyond the end of the text.
    #[inline]
    pub fn fix_for_text(&mut self, text: &str) {
        self.cursor = text.floor_char_boundary(self.cursor);
        self.anchor = text.floor_char_boundary(self.anchor);
    }
}
