// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Tools for text selection

use crate::geom::{Rect, Vec2};
use crate::theme::Text;
use cast::CastFloat;
use kas_text::{TextDisplay, format::FormattableText};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Action used by [`crate::event::components::TextInput`]
#[derive(Default)]
pub struct SelectionAction {
    pub anchor: bool,
    pub clear: bool,
    pub repeats: u32,
}

impl SelectionAction {
    /// Construct
    pub fn new(anchor: bool, clear: bool, repeats: u32) -> Self {
        SelectionAction {
            anchor,
            clear,
            repeats,
        }
    }
}

/// Text-selection logic
///
/// This struct holds the index of the edit cursor and selection position, which
/// together form a range. There is no requirement on the order of these two
/// positions. Each may be adjusted independently.
///
/// Additionally, this struct holds the selection anchor index. This usually
/// equals the selection index, but when using double-click or triple-click
/// selection, the anchor represents the initially-clicked position while the
/// selection index represents the expanded position.
#[derive(Clone, Debug, Default)]
pub struct SelectionHelper {
    edit: usize,
    sel: usize,
    anchor: usize,
}

impl SelectionHelper {
    /// Construct from `(edit, selection)` positions
    ///
    /// The anchor position is set to the selection position.
    pub fn new(edit: usize, selection: usize) -> Self {
        SelectionHelper {
            edit,
            sel: selection,
            anchor: selection,
        }
    }

    /// Reset to the default state
    ///
    /// All positions are set to 0.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// True if the selection index equals the cursor index
    pub fn is_empty(&self) -> bool {
        self.edit == self.sel
    }
    /// Clear selection without changing the edit index
    pub fn set_empty(&mut self) {
        self.sel = self.edit;
        self.anchor = self.edit;
    }

    /// Set the cursor index and clear the selection
    pub fn set_all(&mut self, index: usize) {
        self.edit = index;
        self.sel = index;
        self.anchor = index;
    }

    /// Get the cursor index
    pub fn edit_index(&self) -> usize {
        self.edit
    }
    /// Set the cursor index without adjusting the selection index
    pub fn set_edit_index(&mut self, index: usize) {
        self.edit = index;
    }

    /// Get the selection index
    pub fn sel_index(&self) -> usize {
        self.sel
    }
    /// Set the selection index without adjusting the edit index
    ///
    /// The anchor index is also set to the selection index.
    pub fn set_sel_index(&mut self, index: usize) {
        self.sel = index;
        self.anchor = index;
    }
    /// Set the selection index only
    ///
    /// Prefer [`Self::set_sel_index`] unless you know you don't want to set the anchor.
    pub fn set_sel_index_only(&mut self, index: usize) {
        self.sel = index;
    }

    /// Apply new limit to the maximum length
    ///
    /// Call this method if the string changes under the selection to ensure
    /// that the selection does not exceed the length of the new string.
    pub fn set_max_len(&mut self, len: usize) {
        self.edit = self.edit.min(len);
        self.sel = self.sel.min(len);
        self.anchor = self.anchor.min(len);
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

    /// Set the anchor position to the start of the selection range
    pub fn set_anchor_to_range_start(&mut self) {
        self.anchor = self.range().start;
    }

    /// Get the range from the anchor position to the edit position
    ///
    /// This is used following [`Self::set_anchor_to_range_start`] to get the
    /// IME pre-edit range.
    pub fn anchor_to_edit_range(&self) -> Range<usize> {
        debug_assert!(self.anchor <= self.edit);
        self.anchor..self.edit
    }

    /// Expand the selection from the range between edit index and anchor index
    ///
    /// This moves both edit index and selection index. To obtain repeatable behaviour,
    /// first set `self.anchor_pos`.
    /// then before each time this method is called set the edit position.
    ///
    /// If `repeats <= 2`, the selection is expanded by words, otherwise it is
    /// expanded by lines. Line expansion only works if text is line-wrapped
    /// (layout has been solved).
    fn expand<T: FormattableText>(&mut self, text: &Text<T>, repeats: u32) {
        let string = text.as_str();
        let mut range = self.edit..self.anchor;
        if range.start > range.end {
            std::mem::swap(&mut range.start, &mut range.end);
        }
        let (mut start, mut end);
        if repeats <= 2 {
            end = string[range.start..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| range.start + i)
                .unwrap_or(string.len());
            start = string[0..end]
                .split_word_bound_indices()
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(0);
            end = string[start..]
                .split_word_bound_indices()
                .find_map(|(index, _)| {
                    let pos = start + index;
                    (pos >= range.end).then_some(pos)
                })
                .unwrap_or(string.len());
        } else {
            start = match text.find_line(range.start) {
                Ok(Some(r)) => r.1.start,
                _ => 0,
            };
            end = match text.find_line(range.end) {
                Ok(Some(r)) => r.1.end,
                _ => string.len(),
            };
        }

        if self.edit < self.sel {
            std::mem::swap(&mut start, &mut end);
        }
        self.sel = start;
        self.edit = end;
    }

    /// Handle an action
    pub fn action<T: FormattableText>(&mut self, text: &Text<T>, action: SelectionAction) {
        if action.anchor {
            self.anchor = self.edit;
        }
        if action.clear {
            self.set_empty();
        }
        if action.repeats > 1 {
            self.expand(text, action.repeats);
        }
    }

    /// Return a [`Rect`] encompassing the cursor(s) and selection
    pub fn cursor_rect(&self, text: &TextDisplay) -> Option<Rect> {
        let (m1, m2);
        if self.sel == self.edit {
            let mut iter = text.text_glyph_pos(self.edit);
            m1 = iter.next();
            m2 = iter.next();
        } else if self.sel < self.edit {
            m1 = text.text_glyph_pos(self.sel).next_back();
            m2 = text.text_glyph_pos(self.edit).next();
        } else {
            m1 = text.text_glyph_pos(self.edit).next_back();
            m2 = text.text_glyph_pos(self.sel).next();
        }

        if let Some((c1, c2)) = m1.zip(m2) {
            let left = c1.pos.0.min(c2.pos.0);
            let right = c1.pos.0.max(c2.pos.0);
            let top = (c1.pos.1 - c1.ascent).min(c2.pos.1 - c2.ascent);
            let bottom = (c1.pos.1 - c1.descent).max(c2.pos.1 - c2.ascent);
            let p1 = Vec2(left, top).cast_floor();
            let p2 = Vec2(right, bottom).cast_ceil();
            Some(Rect::from_coords(p1, p2))
        } else if let Some(c) = m1.or(m2) {
            let p1 = Vec2(c.pos.0, c.pos.1 - c.ascent).cast_floor();
            let p2 = Vec2(c.pos.0, c.pos.1 - c.descent).cast_ceil();
            Some(Rect::from_coords(p1, p2))
        } else {
            None
        }
    }
}
