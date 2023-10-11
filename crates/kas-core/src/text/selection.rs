// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Tools for text selection

use super::{TextApi, TextApiExt};
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
/// This struct holds an "edit pos" and a "selection pos", which together form
/// a range. There is no requirement on the order of these two positions. Each
/// may be adjusted independently.
#[derive(Clone, Debug, Default)]
pub struct SelectionHelper {
    edit_pos: usize,
    sel_pos: usize,
    anchor_pos: usize,
}

impl SelectionHelper {
    /// Construct from `(edit, selection)` positions
    ///
    /// The anchor position is set to the selection position.
    pub fn new(edit_pos: usize, sel_pos: usize) -> Self {
        let anchor_pos = sel_pos;
        SelectionHelper {
            edit_pos,
            sel_pos,
            anchor_pos,
        }
    }

    /// Reset to the default state
    ///
    /// All positions are set to 0.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// True if the edit pos equals the selection pos
    pub fn is_empty(&self) -> bool {
        self.edit_pos == self.sel_pos
    }
    /// Clear selection without changing edit pos
    pub fn set_empty(&mut self) {
        self.sel_pos = self.edit_pos;
    }

    /// Set both edit and selection positions to this value
    pub fn set_pos(&mut self, pos: usize) {
        self.edit_pos = pos;
        self.sel_pos = pos;
    }

    /// Get the edit pos
    pub fn edit_pos(&self) -> usize {
        self.edit_pos
    }
    /// Set the edit pos without adjusting the selection pos
    pub fn set_edit_pos(&mut self, pos: usize) {
        self.edit_pos = pos;
    }

    /// Get the selection pos
    pub fn sel_pos(&self) -> usize {
        self.sel_pos
    }
    /// Set the selection pos without adjusting the edit pos
    pub fn set_sel_pos(&mut self, pos: usize) {
        self.sel_pos = pos;
    }

    /// Apply new limit to the maximum length
    ///
    /// Call this method if the string changes under the selection to ensure
    /// that the selection does not exceed the length of the new string.
    pub fn set_max_len(&mut self, len: usize) {
        self.edit_pos = self.edit_pos.min(len);
        self.sel_pos = self.sel_pos.min(len);
        self.anchor_pos = self.anchor_pos.min(len);
    }

    /// Construct a range from the edit pos and selection pos
    ///
    /// The range is from the minimum of (edit pos, selection pos) to the
    /// maximum of the two.
    pub fn range(&self) -> Range<usize> {
        let mut range = self.edit_pos..self.sel_pos;
        if range.start > range.end {
            std::mem::swap(&mut range.start, &mut range.end);
        }
        range
    }

    /// Set the anchor position from the edit position
    pub fn set_anchor(&mut self) {
        self.anchor_pos = self.edit_pos;
    }

    /// Expand the selection from the range between edit pos and anchor pos
    ///
    /// This moves both edit pos and sel pos. To obtain repeatable behaviour,
    /// first use [`SelectionHelper::set_anchor`] to set the anchor position,
    /// then before each time this method is called set the edit position.
    ///
    /// If `repeats <= 2`, the selection is expanded by words, otherwise it is
    /// expanded by lines. Line expansion only works if text is line-wrapped
    /// (layout has been solved).
    pub fn expand<T: TextApi>(&mut self, text: &T, repeats: u32) {
        let string = text.as_str();
        let mut range = self.edit_pos..self.anchor_pos;
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

        if self.edit_pos < self.sel_pos {
            std::mem::swap(&mut start, &mut end);
        }
        self.sel_pos = start;
        self.edit_pos = end;
    }

    /// Handle an action
    pub fn action<T: TextApi>(&mut self, text: &T, action: SelectionAction) {
        if action.anchor {
            self.set_anchor();
        }
        if action.clear {
            self.set_empty();
        }
        if action.repeats > 1 {
            self.expand(text, action.repeats);
        }
    }
}
