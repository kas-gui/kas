// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text editor component

use super::*;
use kas::event::components::TextInput;
use kas::event::{ImePurpose, ImeSurroundingText, Scroll};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::{CursorRange, SelectionHelper};
use kas::theme::{Text, TextClass};

/// Editor component
#[autoimpl(Debug)]
pub struct Editor {
    // TODO(opt): id, pos are duplicated here since macros don't let us put the core here
    pub(super) id: Id,
    pub(super) pos: Coord,
    pub(super) editable: bool,
    pub(super) text: Text<String>,
    pub(super) selection: SelectionHelper,
    pub(super) edit_x_coord: Option<f32>,
    pub(super) has_key_focus: bool,
    pub(super) current: CurrentAction,
    pub(super) error_state: bool,
    pub(super) input_handler: TextInput,
}

/// API for use by `EditField`
impl Editor {
    /// Construct a default instance (empty string)
    #[inline]
    pub(super) fn new() -> Self {
        Editor {
            id: Id::default(),
            pos: Coord::ZERO,
            editable: true,
            text: Text::new(String::new(), TextClass::Editor, false),
            selection: Default::default(),
            edit_x_coord: None,
            has_key_focus: false,
            current: CurrentAction::None,
            error_state: false,
            input_handler: Default::default(),
        }
    }

    /// Construct from a string
    #[inline]
    pub(super) fn from<S: ToString>(text: S) -> Self {
        let text = text.to_string();
        let len = text.len();
        Editor {
            text: Text::new(text, TextClass::Editor, false),
            selection: SelectionHelper::from(len),
            ..Editor::new()
        }
    }

    /// Enable IME if not already enabled
    pub(super) fn enable_ime(&mut self, cx: &mut EventCx) {
        if self.current.is_none() {
            let hint = Default::default();
            let purpose = ImePurpose::Normal;
            let surrounding_text = self.ime_surrounding_text();
            cx.request_ime_focus(self.id.clone(), hint, purpose, surrounding_text);
        }
    }

    /// Cancel on-going selection and IME actions
    ///
    /// This should be called if e.g. key-input interrupts the current
    /// action.
    pub(super) fn cancel_selection_and_ime(&mut self, cx: &mut EventState) {
        if self.current == CurrentAction::Selection {
            self.input_handler.stop_selecting();
            self.current = CurrentAction::None;
        } else if self.current.is_ime_enabled() {
            self.clear_ime();
            cx.cancel_ime_focus(&self.id);
        }
    }

    /// Clean up IME state
    ///
    /// One should also call [`EventCx::cancel_ime_focus`] unless this is
    /// implied.
    pub(super) fn clear_ime(&mut self) {
        if self.current.is_ime_enabled() {
            let action = std::mem::replace(&mut self.current, CurrentAction::None);
            if let CurrentAction::ImePreedit { edit_range } = action {
                self.selection.set_cursor(edit_range.start.cast());
                self.text.replace_range(edit_range.cast(), "");
            }
        }
    }

    pub(super) fn ime_surrounding_text(&self) -> Option<ImeSurroundingText> {
        const MAX_TEXT_BYTES: usize = ImeSurroundingText::MAX_TEXT_BYTES;

        let sel_range = self.selection.range();
        let edit_range = match self.current.clone() {
            CurrentAction::ImePreedit { edit_range } => Some(edit_range.cast()),
            _ => None,
        };
        let mut range = edit_range.clone().unwrap_or(sel_range);
        let initial_range = range.clone();
        let edit_len = edit_range.clone().map(|r| r.len()).unwrap_or(0);

        if let Ok(Some((_, line_range))) = self.text.find_line(range.start) {
            range.start = line_range.start;
        }
        if let Ok(Some((_, line_range))) = self.text.find_line(range.end) {
            range.end = line_range.end;
        }

        if range.len() - edit_len > MAX_TEXT_BYTES {
            range.end = range.end.min(initial_range.end + MAX_TEXT_BYTES / 2);
            while !self.text.as_str().is_char_boundary(range.end) {
                range.end -= 1;
            }

            if range.len() - edit_len > MAX_TEXT_BYTES {
                range.start = range.start.max(initial_range.start - MAX_TEXT_BYTES / 2);
                while !self.text.as_str().is_char_boundary(range.start) {
                    range.start += 1;
                }
            }
        }

        let start = range.start;
        let mut text = String::with_capacity(range.len() - edit_len);
        if let Some(er) = edit_range {
            text.push_str(&self.text.as_str()[range.start..er.start]);
            text.push_str(&self.text.as_str()[er.end..range.end]);
        } else {
            text = self.text.as_str()[range].to_string();
        }

        let cursor = self.selection.edit_index().saturating_sub(start);
        // Terminology difference: our sel_index is called 'anchor'
        // SelectionHelper::anchor is not the same thing.
        let sel_index = self.selection.sel_index().saturating_sub(start);
        ImeSurroundingText::new(text, cursor, sel_index)
            .inspect_err(|err| {
                // TODO: use Display for err not Debug
                log::warn!("Editor::ime_surrounding_text failed: {err:?}")
            })
            .ok()
    }

    /// Call to set IME position only while IME is active
    pub(super) fn set_ime_cursor_area(&self, cx: &mut EventState) {
        if let Ok(text) = self.text.display() {
            let range = match self.current.clone() {
                CurrentAction::ImeStart => self.selection.range(),
                CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                _ => return,
            };

            let (m1, m2);
            if range.is_empty() {
                let mut iter = text.text_glyph_pos(range.start);
                m1 = iter.next();
                m2 = iter.next();
            } else {
                m1 = text.text_glyph_pos(range.start).next_back();
                m2 = text.text_glyph_pos(range.end).next();
            }

            let rect = if let Some((c1, c2)) = m1.zip(m2) {
                let left = c1.pos.0.min(c2.pos.0);
                let right = c1.pos.0.max(c2.pos.0);
                let top = (c1.pos.1 - c1.ascent).min(c2.pos.1 - c2.ascent);
                let bottom = (c1.pos.1 - c1.descent).max(c2.pos.1 - c2.ascent);
                let p1 = Vec2(left, top).cast_floor();
                let p2 = Vec2(right, bottom).cast_ceil();
                Rect::from_coords(p1, p2)
            } else if let Some(c) = m1.or(m2) {
                let p1 = Vec2(c.pos.0, c.pos.1 - c.ascent).cast_floor();
                let p2 = Vec2(c.pos.0, c.pos.1 - c.descent).cast_ceil();
                Rect::from_coords(p1, p2)
            } else {
                return;
            };

            cx.set_ime_cursor_area(&self.id, rect + Offset::conv(self.pos));
        }
    }

    // Set cursor position. It is assumed that the text has not changed.
    pub(super) fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos = (coord - self.pos).cast();
        if let Ok(index) = self.text.text_index_nearest(rel_pos) {
            if index != self.selection.edit_index() {
                self.selection.set_edit_index(index);
                self.set_view_offset_from_cursor(cx);
                self.edit_x_coord = None;
                cx.redraw();
            }
        }
    }

    /// Set primary clipboard (mouse buffer) contents from selection
    pub(super) fn set_primary(&self, cx: &mut EventCx) {
        if self.has_key_focus && !self.selection.is_empty() && cx.has_primary() {
            let range = self.selection.range();
            cx.set_primary(String::from(&self.text.as_str()[range]));
        }
    }

    /// Update view_offset after the cursor index changes
    ///
    /// It is assumed that the text has not changed.
    ///
    /// A redraw is assumed since the cursor moved.
    pub(super) fn set_view_offset_from_cursor(&mut self, cx: &mut EventCx) {
        let cursor = self.selection.edit_index();
        if let Some(marker) = self
            .text
            .text_glyph_pos(cursor)
            .ok()
            .and_then(|mut m| m.next_back())
        {
            let y0 = (marker.pos.1 - marker.ascent).cast_floor();
            let pos = self.pos + Offset(marker.pos.0.cast_nearest(), y0);
            let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
            cx.set_scroll(Scroll::Rect(Rect { pos, size }));
        }
    }
}

/// API for use by `EditGuard` implementations
impl Editor {
    /// Get a reference to the widget's identifier
    #[inline]
    pub fn id_ref(&self) -> &Id {
        &self.id
    }

    /// Get the widget's identifier
    #[inline]
    pub fn id(&self) -> Id {
        self.id.clone()
    }

    /// Get text contents
    #[inline]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    /// Get the text contents as a `String`
    #[inline]
    pub fn clone_string(&self) -> String {
        self.text.clone_string()
    }

    /// Access the cursor index / selection range
    #[inline]
    pub fn cursor_range(&self) -> CursorRange {
        *self.selection
    }

    /// Set the cursor index / range
    #[inline]
    pub fn set_cursor_range(&mut self, range: impl Into<CursorRange>) {
        self.edit_x_coord = None;
        self.selection = range.into().into();
    }

    /// Get whether this `EditField` is editable
    #[inline]
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether this `EditField` is editable
    #[inline]
    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// True if the editor uses multi-line mode
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.text.wrap()
    }

    /// Get the text class used
    #[inline]
    pub fn class(&self) -> TextClass {
        self.text.class()
    }

    /// Get whether the widget has edit focus
    ///
    /// This is true when the widget is editable and has keyboard focus.
    #[inline]
    pub fn has_edit_focus(&self) -> bool {
        self.editable && self.has_key_focus
    }

    /// Get whether the input state is erroneous
    #[inline]
    pub fn has_error(&self) -> bool {
        self.error_state
    }

    /// Set the error state
    ///
    /// When true, the input field's background is drawn red.
    /// This state is cleared by [`Self::set_string`].
    // TODO: possibly change type to Option<String> and display the error
    pub fn set_error_state(&mut self, cx: &mut EventState, error_state: bool) {
        self.error_state = error_state;
        cx.redraw(&self.id);
    }
}
