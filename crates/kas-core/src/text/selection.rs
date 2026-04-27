// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Tools for text selection

use std::ops::Range;

/// Cursor index and selection range
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorRange {
    /// The start or end of the selection.
    pub anchor: usize,
    /// The cursor (edit) index.
    pub cursor: usize,
}

impl From<usize> for CursorRange {
    #[inline]
    fn from(index: usize) -> Self {
        CursorRange {
            anchor: index,
            cursor: index,
        }
    }
}

impl From<Range<usize>> for CursorRange {
    #[inline]
    fn from(range: Range<usize>) -> Self {
        CursorRange {
            anchor: range.start,
            cursor: range.end,
        }
    }
}

impl CursorRange {
    /// True if the selection index equals the cursor index
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cursor == self.anchor
    }

    /// Convert to a [`Range`], increasing
    ///
    /// The return value has `range.start <= range.end`.
    pub fn to_range(&self) -> Range<usize> {
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
    pub fn set_position(&mut self, index: usize) {
        self.anchor = index;
        self.cursor = index;
    }

    /// Apply new limit to the maximum length
    ///
    /// Call this method if the string changes under the selection to ensure
    /// that the selection does not exceed the length of the new string.
    #[inline]
    pub fn set_max_len(&mut self, len: usize) {
        self.cursor = self.cursor.min(len);
        self.anchor = self.anchor.min(len);
    }

    /// Adjust all indices for a deletion from the source text
    pub fn delete_range(&mut self, range: Range<usize>) {
        let len = range.len();
        let adjust = |index: usize| -> usize {
            if index >= range.end {
                index - len
            } else if index > range.start {
                range.start
            } else {
                index
            }
        };
        self.cursor = adjust(self.cursor);
        self.anchor = adjust(self.anchor);
    }
}
