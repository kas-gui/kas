// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Tools for text selection

use kas_macros::autoimpl;
use std::ops::Range;

/// Cursor index / selection range
///
/// This is essentially a pair of indices: the selection index and the edit
/// index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorRange {
    sel: usize,
    edit: usize,
}

impl From<usize> for CursorRange {
    #[inline]
    fn from(index: usize) -> Self {
        CursorRange {
            sel: index,
            edit: index,
        }
    }
}

impl From<Range<usize>> for CursorRange {
    #[inline]
    fn from(range: Range<usize>) -> Self {
        CursorRange {
            sel: range.start,
            edit: range.end,
        }
    }
}

impl CursorRange {
    /// Construct from `(selection, edit)` positions
    ///
    /// Constructs as a range, with the cursor at the `edit` position.
    ///
    /// See also:
    ///
    /// - `Default`: an empty cursor at index 0
    /// - `From<usize>`: construct from an index (empty selection)
    /// - `From<Range<usize>>`: construct from a range (potentially non-empty
    ///   selection; edit position is set to the range's end)
    #[inline]
    pub fn new(sel: usize, edit: usize) -> Self {
        CursorRange { sel, edit }
    }

    /// True if the selection index equals the cursor index
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edit == self.sel
    }

    /// Get the selection index
    pub fn sel_index(&self) -> usize {
        self.sel
    }

    /// Get the edit index (also known as the cursor)
    pub fn edit_index(&self) -> usize {
        self.edit
    }

    /// Get the selection range
    ///
    /// This range is from the edit index to the selection index or reversed,
    /// whichever is increasing.
    pub fn range(&self) -> Range<usize> {
        let mut range = self.edit..self.sel;
        if range.start > range.end {
            std::mem::swap(&mut range.start, &mut range.end);
        }
        range
    }
}

/// Text-selection logic
///
/// This struct holds a [`CursorRange`]. There is no requirement on the order of these two
/// positions. Each may be adjusted independently.
///
/// Additionally, this struct holds the selection anchor index. This usually
/// equals the selection index, but when using double-click or triple-click
/// selection, the anchor represents the initially-clicked position while the
/// selection index represents the expanded position.
#[derive(Clone, Debug, Default)]
#[autoimpl(Deref, DerefMut using self.cursor)]
pub struct SelectionHelper {
    cursor: CursorRange,
    anchor: usize,
}

impl<T: Into<CursorRange>> From<T> for SelectionHelper {
    fn from(x: T) -> Self {
        let cursor = x.into();
        SelectionHelper {
            cursor,
            anchor: cursor.sel,
        }
    }
}

impl SelectionHelper {
    /// Clear selection
    ///
    /// Sets the selection and anchor indices to the edit index.
    #[inline]
    pub fn clear_selection(&mut self) {
        self.sel = self.edit;
        self.anchor = self.edit;
    }

    /// Set the cursor position and clear the selection
    ///
    /// All three indices are set to `index`.
    #[inline]
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor.sel = index;
        self.cursor.edit = index;
        self.anchor = index;
    }

    /// Set the cursor index
    ///
    /// Does not adjust the selection index or anchor.
    #[inline]
    pub fn set_edit_index(&mut self, index: usize) {
        self.edit = index;
    }

    /// Set the selection index and anchor
    ///
    /// Does not adjust the cursor (edit index).
    #[inline]
    pub fn set_sel_index(&mut self, index: usize) {
        self.sel = index;
        self.anchor = index;
    }

    /// Apply new limit to the maximum length
    ///
    /// Call this method if the string changes under the selection to ensure
    /// that the selection does not exceed the length of the new string.
    #[inline]
    pub fn set_max_len(&mut self, len: usize) {
        self.edit = self.edit.min(len);
        self.sel = self.sel.min(len);
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
        self.edit = adjust(self.edit);
        self.sel = adjust(self.sel);
        self.anchor = adjust(self.anchor);
    }
}
