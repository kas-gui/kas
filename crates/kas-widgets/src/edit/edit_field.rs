// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{CursorIcon, ElementState, FocusSource, PhysicalKey, Scroll};
use kas::event::{Ime, ImePurpose, ImeSurroundingText};
use kas::geom::Vec2;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::text::{CursorRange, Effect, EffectFlags, NotReady, SelectionHelper};
use kas::theme::{Text, TextClass};
use kas::util::UndoStack;
use std::fmt::{Debug, Display};
use std::ops::Range;
use std::str::FromStr;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

#[impl_self]
mod EditField {
    /// A text-edit field (single- or multi-line)
    ///
    /// The [`EditBox`] widget should be preferred in most cases; this widget
    /// is a component of `EditBox` and has some special behaviour.
    ///
    /// By default, the editor supports a single-line only;
    /// [`Self::with_multi_line`] and [`Self::with_class`] can be used to change this.
    ///
    /// ### Event handling
    ///
    /// This widget attempts to handle all standard text-editor input and scroll
    /// events.
    ///
    /// Key events for moving the edit cursor (e.g. arrow keys) are consumed
    /// only if the edit cursor is moved while key events for adjusting or using
    /// the selection (e.g. `Command::Copy` and `Command::Deselect`)
    /// are consumed only when a selection exists. In contrast, key events for
    /// inserting or deleting text are always consumed.
    ///
    /// [`Command::Enter`] inserts a line break in multi-line mode, but in
    /// single-line mode or if the <kbd>Shift</kbd> key is held it is treated
    /// the same as [`Command::Activate`].
    ///
    /// ### Performance and limitations
    ///
    /// Text representation is via a single [`String`]. Edit operations are
    /// `O(n)` where `n` is the length of text (with text layout algorithms
    /// having greater cost than copying bytes in the backing [`String`]).
    /// This isn't necessarily *slow*; when run with optimizations the type can
    /// handle type-setting around 20kB of UTF-8 in under 10ms (with significant
    /// scope for optimization, given that currently layout is re-run from
    /// scratch on each key stroke). Regardless, this approach is not designed
    /// to scale to handle large documents via a single `EditField` widget.
    ///
    /// ### Messages
    ///
    /// [`SetValueText`] may be used to replace the entire text and
    /// [`ReplaceSelectedText`] may be used to replace selected text, where
    /// [`Self::is_editable`]. This triggers the action handlers
    /// [`EditGuard::edit`] followed by [`EditGuard::activate`].
    ///
    /// ### Special behaviour
    ///
    /// This is a [`Viewport`] widget.
    #[autoimpl(Debug where G: trait)]
    #[widget]
    pub struct EditField<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        editable: bool,
        width: (f32, f32),
        lines: (f32, f32),
        text: Text<String>,
        selection: SelectionHelper,
        edit_x_coord: Option<f32>,
        last_edit: Option<EditOp>,
        undo_stack: UndoStack<(String, CursorRange)>,
        has_key_focus: bool,
        current: CurrentAction,
        error_state: bool,
        input_handler: TextInput,
        /// The associated [`EditGuard`] implementation
        pub guard: G,
    }

    impl Layout for Self {
        #[inline]
        fn rect(&self) -> Rect {
            self.text.rect()
        }

        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, mut ideal): (i32, i32);
            if axis.is_horizontal() {
                let dpem = cx.dpem(self.text.class());
                min = (self.width.0 * dpem).cast_ceil();
                ideal = (self.width.1 * dpem).cast_ceil();
            } else if let Some(width) = axis.other() {
                // Use the height of the first line as a reference
                let height = self
                    .text
                    .measure_height(width.cast(), std::num::NonZero::new(1));
                min = (self.lines.0 * height).cast_ceil();
                ideal = (self.lines.1 * height).cast_ceil();
            } else {
                unreachable!()
            };

            let rules = self.text.size_rules(cx, axis);
            ideal = ideal.max(rules.ideal_size());

            let stretch = if axis.is_horizontal() || self.multi_line() {
                Stretch::High
            } else {
                Stretch::None
            };
            SizeRules::new(min, ideal, stretch).with_margins(cx.text_margins().extract(axis))
        }

        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, mut hints: AlignHints) {
            hints.vert = Some(if self.multi_line() {
                Align::Default
            } else {
                Align::Center
            });
            self.text.set_rect(cx, rect, hints);
            self.text.ensure_no_left_overhang();
            if self.current.is_ime() {
                self.set_ime_cursor_area(cx);
            }
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            if let Ok((tl, br)) = self.text.bounding_box() {
                (br - tl).cast_ceil()
            } else {
                Size::ZERO
            }
        }

        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            let pos = self.rect().pos - offset;

            if let CurrentAction::ImePreedit { edit_range } = self.current.clone() {
                // TODO: combine underline with selection highlight
                let effects = [
                    Effect {
                        start: 0,
                        e: 0,
                        flags: Default::default(),
                    },
                    Effect {
                        start: edit_range.start,
                        e: 0,
                        flags: EffectFlags::UNDERLINE,
                    },
                    Effect {
                        start: edit_range.end,
                        e: 0,
                        flags: Default::default(),
                    },
                ];
                draw.text_with_effects(pos, rect, &self.text, &[], &effects);
            } else {
                draw.text_with_selection(pos, rect, &self.text, self.selection.range());
            }

            if self.editable && draw.ev_state().has_key_focus(self.id_ref()).0 {
                draw.text_cursor(pos, rect, &self.text, self.selection.edit_index());
            }
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::TextInput {
                text: self.text.as_str(),
                multi_line: self.multi_line(),
                cursor: self.cursor_range(),
            }
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = G::Data;

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }

        #[inline]
        fn mouse_over_icon(&self) -> Option<CursorIcon> {
            Some(CursorIcon::Text)
        }

        fn configure(&mut self, cx: &mut ConfigCx) {
            self.text.configure(&mut cx.size_cx());
            G::configure(self, cx);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &G::Data) {
            let size = self.content_size();
            G::update(self, cx, data);
            if size != self.content_size() {
                cx.resize();
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &G::Data, event: Event) -> IsUsed {
            match event {
                Event::NavFocus(source) if source == FocusSource::Key => {
                    if !self.input_handler.is_selecting() {
                        self.request_key_focus(cx, source);
                    }
                    Used
                }
                Event::NavFocus(_) => Used,
                Event::LostNavFocus => Used,
                Event::SelFocus(source) => {
                    // NOTE: sel focus implies key focus since we only request
                    // the latter. We must set before calling self.set_primary.
                    self.has_key_focus = true;
                    if source == FocusSource::Pointer {
                        self.set_primary(cx);
                    }
                    Used
                }
                Event::KeyFocus => {
                    self.has_key_focus = true;
                    self.set_view_offset_from_cursor(cx);
                    G::focus_gained(self, cx, data);
                    self.enable_ime(cx);
                    Used
                }
                Event::LostKeyFocus => {
                    self.has_key_focus = false;
                    cx.redraw();
                    G::focus_lost(self, cx, data);
                    Used
                }
                Event::LostSelFocus => {
                    // NOTE: we can assume that we will receive Ime::Disabled if IME is active
                    if !self.selection.is_empty() {
                        self.save_undo_state(None);
                        self.selection.set_empty();
                    }
                    self.input_handler.stop_selecting();
                    cx.redraw();
                    Used
                }
                Event::Command(cmd, code) => match self.control_key(cx, data, cmd, code) {
                    Ok(r) => r,
                    Err(NotReady) => Used,
                },
                Event::Key(event, false) if event.state == ElementState::Pressed => {
                    if let Some(text) = &event.text {
                        self.save_undo_state(Some(EditOp::KeyInput));
                        let used = self.received_text(cx, text);
                        G::edit(self, cx, data);
                        used
                    } else {
                        let opt_cmd = cx
                            .config()
                            .shortcuts()
                            .try_match(cx.modifiers(), &event.key_without_modifiers);
                        if let Some(cmd) = opt_cmd {
                            match self.control_key(cx, data, cmd, Some(event.physical_key)) {
                                Ok(r) => r,
                                Err(NotReady) => Used,
                            }
                        } else {
                            Unused
                        }
                    }
                }
                Event::Ime(ime) => match ime {
                    Ime::Enabled => {
                        match self.current {
                            CurrentAction::None => {
                                self.current = CurrentAction::ImeStart;
                                self.set_ime_cursor_area(cx);
                            }
                            CurrentAction::ImeStart | CurrentAction::ImePreedit { .. } => {
                                // already enabled
                            }
                            CurrentAction::Selection => {
                                // Do not interrupt selection
                                cx.cancel_ime_focus(self.id_ref());
                            }
                        }
                        Used
                    }
                    Ime::Disabled => {
                        self.clear_ime();
                        Used
                    }
                    Ime::Preedit { text, cursor } => {
                        self.save_undo_state(None);
                        let mut edit_range = match self.current.clone() {
                            CurrentAction::ImeStart if cursor.is_some() => self.selection.range(),
                            CurrentAction::ImeStart => return Used,
                            CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                            _ => return Used,
                        };

                        self.text.replace_range(edit_range.clone(), text);
                        edit_range.end = edit_range.start + text.len();
                        if let Some((start, end)) = cursor {
                            self.selection.set_sel_index_only(edit_range.start + start);
                            self.selection.set_edit_index(edit_range.start + end);
                        } else {
                            self.selection.set_cursor(edit_range.start + text.len());
                        }

                        self.current = CurrentAction::ImePreedit {
                            edit_range: edit_range.cast(),
                        };
                        self.edit_x_coord = None;
                        self.prepare_text(cx, false);
                        Used
                    }
                    Ime::Commit { text } => {
                        self.save_undo_state(Some(EditOp::Ime));
                        let edit_range = match self.current.clone() {
                            CurrentAction::ImeStart => self.selection.range(),
                            CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                            _ => return Used,
                        };

                        self.text.replace_range(edit_range.clone(), text);
                        self.selection.set_cursor(edit_range.start + text.len());

                        self.current = CurrentAction::ImePreedit {
                            edit_range: self.selection.range().cast(),
                        };
                        self.edit_x_coord = None;
                        self.prepare_text(cx, false);
                        G::edit(self, cx, data);
                        Used
                    }
                    Ime::DeleteSurrounding {
                        before_bytes,
                        after_bytes,
                    } => {
                        self.save_undo_state(None);
                        let edit_range = match self.current.clone() {
                            CurrentAction::ImeStart => self.selection.range(),
                            CurrentAction::ImePreedit { edit_range } => edit_range.cast(),
                            _ => return Used,
                        };

                        if before_bytes > 0 {
                            let end = edit_range.start;
                            let start = end - before_bytes;
                            if self.as_str().is_char_boundary(start) {
                                self.text.replace_range(start..end, "");
                                self.selection.delete_range(start..end);
                            } else {
                                log::warn!("buggy IME tried to delete range not at char boundary");
                            }
                        }

                        if after_bytes > 0 {
                            let start = edit_range.end;
                            let end = start + after_bytes;
                            if self.as_str().is_char_boundary(end) {
                                self.text.replace_range(start..end, "");
                            } else {
                                log::warn!("buggy IME tried to delete range not at char boundary");
                            }
                        }

                        if let Some(text) = self.ime_surrounding_text() {
                            cx.update_ime_surrounding_text(self.id_ref(), text);
                        }

                        Used
                    }
                },
                Event::PressStart(press) if press.is_tertiary() => {
                    press.grab_click(self.id()).complete(cx)
                }
                Event::PressEnd { press, .. } if press.is_tertiary() => {
                    self.set_cursor_from_coord(cx, press.coord);
                    self.cancel_selection_and_ime(cx);

                    if let Some(content) = cx.get_primary() {
                        self.save_undo_state(Some(EditOp::Clipboard));

                        let index = self.selection.edit_index();
                        let range = self.trim_paste(&content);

                        self.text
                            .replace_range(index..index, &content[range.clone()]);
                        self.selection.set_cursor(index + range.len());
                        self.edit_x_coord = None;
                        self.prepare_text(cx, false);

                        G::edit(self, cx, data);
                    }

                    self.request_key_focus(cx, FocusSource::Pointer);
                    Used
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::Used => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::PressStart {
                        coord,
                        clear,
                        repeats,
                    } => {
                        if self.current.is_ime() {
                            self.clear_ime();
                            cx.cancel_ime_focus(self.id_ref());
                        }
                        self.save_undo_state(Some(EditOp::Cursor));
                        self.current = CurrentAction::Selection;

                        self.set_cursor_from_coord(cx, coord);
                        self.selection.set_anchor(clear);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }

                        self.request_key_focus(cx, FocusSource::Pointer);
                        Used
                    }
                    TextInputAction::PressMove { coord, repeats } => {
                        self.set_cursor_from_coord(cx, coord);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }

                        Used
                    }
                    TextInputAction::PressEnd { coord } => {
                        if self.current.is_ime() {
                            self.clear_ime();
                            cx.cancel_ime_focus(self.id_ref());
                        }
                        if self.current != CurrentAction::Selection {
                            self.set_cursor_from_coord(cx, coord);
                            self.selection.set_empty();
                        }
                        self.current = CurrentAction::None;

                        self.set_primary(cx);
                        self.request_key_focus(cx, FocusSource::Pointer);
                        self.enable_ime(cx);
                        Used
                    }
                },
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &G::Data) {
            if !self.editable {
                return;
            }

            if let Some(SetValueText(string)) = cx.try_pop() {
                self.pre_commit();
                self.set_string(cx, string);
                G::edit(self, cx, data);
                G::activate(self, cx, data);
            } else if let Some(ReplaceSelectedText(text)) = cx.try_pop() {
                self.pre_commit();
                self.replace_selected_text(cx, &text);
                G::edit(self, cx, data);
                G::activate(self, cx, data);
            }
        }
    }

    impl Default for Self
    where
        G: Default,
    {
        #[inline]
        fn default() -> Self {
            EditField::new(G::default())
        }
    }

    impl Self {
        /// Construct an `EditBox` with an [`EditGuard`]
        #[inline]
        pub fn new(guard: G) -> EditField<G> {
            EditField {
                core: Default::default(),
                editable: true,
                width: (8.0, 16.0),
                lines: (1.0, 1.0),
                text: Text::new(String::new(), TextClass::Editor, false),
                selection: Default::default(),
                edit_x_coord: None,
                last_edit: Some(EditOp::Initial),
                undo_stack: UndoStack::new(),
                has_key_focus: false,
                current: CurrentAction::None,
                error_state: false,
                input_handler: Default::default(),
                guard,
            }
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

        /// Clear text contents and undo history
        ///
        /// This method does not call any [`EditGuard`] actions; consider also
        /// calling [`Self::call_guard_edit`].
        #[inline]
        pub fn clear(&mut self, cx: &mut EventState) {
            self.last_edit = Some(EditOp::Initial);
            self.undo_stack.clear();
            self.set_string(cx, String::new());
        }

        /// Commit outstanding changes to the undo history
        ///
        /// Call this *before* changing the text with `set_str` or `set_string`
        /// to commit changes to the undo history.
        #[inline]
        pub fn pre_commit(&mut self) {
            self.save_undo_state(Some(EditOp::Synthetic));
        }

        /// Set text contents from a `str`
        ///
        /// This does not interact with undo history; see also [`Self::clear`],
        /// [`Self::pre_commit`].
        ///
        /// This method does not call any [`EditGuard`] actions; consider also
        /// calling [`Self::call_guard_edit`].
        ///
        /// Returns `true` if the text may have changed.
        #[inline]
        pub fn set_str(&mut self, cx: &mut EventState, text: &str) -> bool {
            if self.text.as_str() != text {
                self.set_string(cx, text.to_string());
                true
            } else {
                false
            }
        }

        /// Set text contents from a `String`
        ///
        /// This does not interact with undo history; see also [`Self::clear`],
        /// [`Self::pre_commit`].
        ///
        /// This method does not call any [`EditGuard`] actions; consider also
        /// calling [`Self::call_guard_edit`].
        ///
        /// Returns `true` if the text is ready and may have changed.
        pub fn set_string(&mut self, cx: &mut EventState, string: String) -> bool {
            self.cancel_selection_and_ime(cx);

            if !self.text.set_string(string) || !self.text.prepare() {
                return false;
            }

            self.selection.set_max_len(self.text.str_len());
            self.edit_x_coord = None;
            cx.redraw(&self);
            self.set_error_state(cx, false);
            true
        }

        /// Replace selected text
        ///
        /// This does not interact with undo history; see also [`Self::clear`],
        /// [`Self::pre_commit`].
        ///
        /// This method does not call any [`EditGuard`] actions; consider also
        /// calling [`Self::call_guard_edit`].
        #[inline]
        pub fn replace_selected_text(&mut self, cx: &mut EventCx, text: &str) {
            self.received_text(cx, text);
        }

        /// Call the [`EditGuard`]'s `activate` method
        #[inline]
        pub fn call_guard_activate(&mut self, cx: &mut EventCx, data: &G::Data) {
            G::activate(self, cx, data);
        }

        /// Call the [`EditGuard`]'s `edit` method
        #[inline]
        pub fn call_guard_edit(&mut self, cx: &mut EventCx, data: &G::Data) {
            G::edit(self, cx, data);
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

        /// Enable IME if not already enabled
        fn enable_ime(&mut self, cx: &mut EventCx) {
            if self.current.is_none() {
                let hint = Default::default();
                let purpose = ImePurpose::Normal;
                let surrounding_text = self.ime_surrounding_text();
                cx.request_ime_focus(self.id(), hint, purpose, surrounding_text);
            }
        }

        /// Cancel on-going selection and IME actions
        ///
        /// This should be called if e.g. key-input interrupts the current
        /// action.
        fn cancel_selection_and_ime(&mut self, cx: &mut EventState) {
            if self.current == CurrentAction::Selection {
                self.input_handler.stop_selecting();
                self.current = CurrentAction::None;
            } else if self.current.is_ime() {
                self.clear_ime();
                cx.cancel_ime_focus(self.id_ref());
            }
        }

        /// Clean up IME state
        ///
        /// One should also call [`EventCx::cancel_ime_focus`] unless this is
        /// implied.
        fn clear_ime(&mut self) {
            if self.current.is_ime() {
                let action = std::mem::replace(&mut self.current, CurrentAction::None);
                if let CurrentAction::ImePreedit { edit_range } = action {
                    self.selection.set_cursor(edit_range.start.cast());
                    self.text.replace_range(edit_range.cast(), "");
                }
            }
        }

        fn ime_surrounding_text(&self) -> Option<ImeSurroundingText> {
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
                while !self.as_str().is_char_boundary(range.end) {
                    range.end -= 1;
                }

                if range.len() - edit_len > MAX_TEXT_BYTES {
                    range.start = range.start.max(initial_range.start - MAX_TEXT_BYTES / 2);
                    while !self.as_str().is_char_boundary(range.start) {
                        range.start += 1;
                    }
                }
            }

            let start = range.start;
            let mut text = String::with_capacity(range.len() - edit_len);
            if let Some(er) = edit_range {
                text.push_str(&self.as_str()[range.start..er.start]);
                text.push_str(&self.as_str()[er.end..range.end]);
            } else {
                text = self.as_str()[range].to_string();
            }

            let cursor = self.selection.edit_index().saturating_sub(start);
            // Terminology difference: our sel_index is called 'anchor'
            // SelectionHelper::anchor is not the same thing.
            let sel_index = self.selection.sel_index().saturating_sub(start);
            ImeSurroundingText::new(text, cursor, sel_index)
                .inspect_err(|err| {
                    // TODO: use Display for err not Debug
                    log::warn!("EditField::ime_surrounding_text failed: {err:?}")
                })
                .ok()
        }

        // Call only if self.ime_focus
        fn set_ime_cursor_area(&self, cx: &mut EventState) {
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

                cx.set_ime_cursor_area(self.id_ref(), rect + Offset::conv(self.rect().pos));
            }
        }
    }
}

impl<A: 'static> EditField<DefaultGuard<A>> {
    /// Construct an `EditField` with the given inital `text` (no event handling)
    #[inline]
    pub fn text<S: ToString>(text: S) -> Self {
        let text = text.to_string();
        let len = text.len();
        EditField {
            editable: true,
            text: Text::new(text, TextClass::Editor, false),
            selection: SelectionHelper::from(len),
            ..Default::default()
        }
    }

    /// Construct a read-only `EditField` displaying some `String` value
    #[inline]
    pub fn string(value_fn: impl Fn(&A) -> String + 'static) -> EditField<StringGuard<A>> {
        EditField::new(StringGuard::new(value_fn)).with_editable(false)
    }

    /// Construct an `EditField` for a parsable value (e.g. a number)
    ///
    /// On update, `value_fn` is used to extract a value from input data
    /// which is then formatted as a string via [`Display`].
    /// If, however, the input field has focus, the update is ignored.
    ///
    /// On every edit, the guard attempts to parse the field's input as type
    /// `T` via [`FromStr`], caching the result and setting the error state.
    ///
    /// On field activation and focus loss when a `T` value is cached (see
    /// previous paragraph), `on_afl` is used to construct a message to be
    /// emitted via [`EventCx::push`]. The cached value is then cleared to
    /// avoid sending duplicate messages.
    #[inline]
    pub fn parser<T: Debug + Display + FromStr, M: Debug + 'static>(
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
    ) -> EditField<ParseGuard<A, T>> {
        EditField::new(ParseGuard::new(value_fn, msg_fn))
    }

    /// Construct an `EditField` for a parsable value (e.g. a number)
    ///
    /// On update, `value_fn` is used to extract a value from input data
    /// which is then formatted as a string via [`Display`].
    /// If, however, the input field has focus, the update is ignored.
    ///
    /// On every edit, the guard attempts to parse the field's input as type
    /// `T` via [`FromStr`]. On success, the result is converted to a
    /// message via `on_afl` then emitted via [`EventCx::push`].
    pub fn instant_parser<T: Debug + Display + FromStr, M: Debug + 'static>(
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
    ) -> EditField<InstantParseGuard<A, T>> {
        EditField::new(InstantParseGuard::new(value_fn, msg_fn))
    }
}

impl<A: 'static> EditField<StringGuard<A>> {
    /// Assign a message function for a `String` value
    ///
    /// The `msg_fn` is called when the field is activated (<kbd>Enter</kbd>)
    /// and when it loses focus after content is changed.
    ///
    /// This method sets self as editable (see [`Self::with_editable`]).
    #[must_use]
    pub fn with_msg<M>(mut self, msg_fn: impl Fn(&str) -> M + 'static) -> Self
    where
        M: Debug + 'static,
    {
        self.guard = self.guard.with_msg(msg_fn);
        self.editable = true;
        self
    }
}

impl<G: EditGuard> EditField<G> {
    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `EditField`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        debug_assert!(self.current == CurrentAction::None && !self.input_handler.is_selecting());
        let text = text.to_string();
        let len = text.len();
        self.text.set_string(text);
        self.selection.set_cursor(len);
        self
    }

    /// Set whether this `EditField` is editable (inline)
    #[inline]
    #[must_use]
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
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

    /// Set whether this `EditField` uses multi-line mode
    ///
    /// This method does two things:
    ///
    /// -   Changes the text class (see [`Self::with_class`])
    /// -   Changes the vertical height allocation (see [`Self::with_lines`])
    #[inline]
    #[must_use]
    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.text.set_wrap(multi_line);
        self.lines = match multi_line {
            false => (1.0, 1.0),
            true => (4.0, 7.0),
        };
        self
    }

    /// True if the editor uses multi-line mode
    ///
    /// See also: [`Self::with_multi_line`]
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.text.wrap()
    }

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.text.set_class(class);
        self
    }

    /// Get the text class used
    #[inline]
    pub fn class(&self) -> TextClass {
        self.text.class()
    }

    /// Adjust the height allocation
    #[inline]
    pub fn set_lines(&mut self, min_lines: f32, ideal_lines: f32) {
        self.lines = (min_lines, ideal_lines);
    }

    /// Adjust the height allocation (inline)
    #[inline]
    #[must_use]
    pub fn with_lines(mut self, min_lines: f32, ideal_lines: f32) -> Self {
        self.set_lines(min_lines, ideal_lines);
        self
    }

    /// Adjust the width allocation
    #[inline]
    pub fn set_width_em(&mut self, min_em: f32, ideal_em: f32) {
        self.width = (min_em, ideal_em);
    }

    /// Adjust the width allocation (inline)
    #[inline]
    #[must_use]
    pub fn with_width_em(mut self, min_em: f32, ideal_em: f32) -> Self {
        self.set_width_em(min_em, ideal_em);
        self
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
        cx.redraw(self);
    }

    /// Request key focus, if we don't have it or IME
    fn request_key_focus(&self, cx: &mut EventState, source: FocusSource) {
        if !self.has_key_focus && !self.current.is_ime() {
            cx.request_key_focus(self.id(), source);
        }
    }

    /// Call before an edit to (potentially) commit current state based on last_edit
    ///
    /// Call with [`None`] to force commit of any uncommitted changes.
    fn save_undo_state(&mut self, edit: Option<EditOp>) {
        if let Some(op) = edit
            && op.try_merge(&mut self.last_edit)
        {
            return;
        }

        self.last_edit = edit;
        self.undo_stack
            .try_push((self.clone_string(), self.cursor_range()));
    }

    fn prepare_text(&mut self, cx: &mut EventCx, force_set_offset: bool) {
        let size = self.content_size();
        if self.text.prepare() {
            self.text.ensure_no_left_overhang();
            cx.redraw();
        }

        let mut set_offset = force_set_offset;
        if size != self.content_size() {
            cx.resize();
            set_offset = true;
        }
        if set_offset {
            self.set_view_offset_from_cursor(cx);
        }
    }

    fn trim_paste(&self, text: &str) -> Range<usize> {
        let mut end = text.len();
        if !self.multi_line() {
            // We cut the content short on control characters and
            // ignore them (preventing line-breaks and ignoring any
            // actions such as recursive-paste).
            for (i, c) in text.char_indices() {
                if c < '\u{20}' || ('\u{7f}'..='\u{9f}').contains(&c) {
                    end = i;
                    break;
                }
            }
        }
        0..end
    }

    fn received_text(&mut self, cx: &mut EventCx, text: &str) -> IsUsed {
        if !self.editable {
            return Unused;
        }
        self.cancel_selection_and_ime(cx);

        let index = self.selection.edit_index();
        let selection = self.selection.range();
        let have_sel = selection.start < selection.end;
        if have_sel {
            self.text.replace_range(selection.clone(), text);
            self.selection.set_cursor(selection.start + text.len());
        } else {
            self.text.insert_str(index, text);
            self.selection.set_cursor(index + text.len());
        }
        self.edit_x_coord = None;

        self.prepare_text(cx, false);
        Used
    }

    fn control_key(
        &mut self,
        cx: &mut EventCx,
        data: &G::Data,
        cmd: Command,
        code: Option<PhysicalKey>,
    ) -> Result<IsUsed, NotReady> {
        let editable = self.editable;
        let mut shift = cx.modifiers().shift_key();
        let mut buf = [0u8; 4];
        let cursor = self.selection.edit_index();
        let len = self.text.str_len();
        let multi_line = self.multi_line();
        let selection = self.selection.range();
        let have_sel = selection.end > selection.start;
        let string;

        enum Action<'a> {
            None,
            Deselect,
            Activate,
            Insert(&'a str, EditOp),
            Delete(Range<usize>, EditOp),
            Move(usize, Option<f32>),
            UndoRedo(bool),
        }

        let action = match cmd {
            Command::Escape | Command::Deselect if !selection.is_empty() => Action::Deselect,
            Command::Activate => Action::Activate,
            Command::Enter if shift || !multi_line => Action::Activate,
            Command::Enter if editable && multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), EditOp::KeyInput)
            }
            // NOTE: we might choose to optionally handle Tab in the future,
            // but without some workaround it prevents keyboard navigation.
            // Command::Tab => Action::Insert('\t'.encode_utf8(&mut buf), EditOp::Insert),
            Command::Left | Command::Home if !shift && have_sel => {
                Action::Move(selection.start, None)
            }
            Command::Left if cursor > 0 => {
                let mut cursor = GraphemeCursor::new(cursor, len, true);
                cursor
                    .prev_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|index| Action::Move(index, None))
                    .unwrap_or(Action::None)
            }
            Command::Right | Command::End if !shift && have_sel => {
                Action::Move(selection.end, None)
            }
            Command::Right if cursor < len => {
                let mut cursor = GraphemeCursor::new(cursor, len, true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|index| Action::Move(index, None))
                    .unwrap_or(Action::None)
            }
            Command::WordLeft if cursor > 0 => {
                let mut iter = self.text.text()[0..cursor].split_word_bound_indices();
                let mut p = iter.next_back().map(|(index, _)| index).unwrap_or(0);
                while self.text.text()[p..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next_back() {
                        p = index;
                    } else {
                        break;
                    }
                }
                Action::Move(p, None)
            }
            Command::WordRight if cursor < len => {
                let mut iter = self.text.text()[cursor..]
                    .split_word_bound_indices()
                    .skip(1);
                let mut p = iter.next().map(|(index, _)| cursor + index).unwrap_or(len);
                while self.text.text()[p..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next() {
                        p = cursor + index;
                    } else {
                        break;
                    }
                }
                Action::Move(p, None)
            }
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Left | Command::Right | Command::WordLeft | Command::WordRight => Action::None,
            Command::Up | Command::Down if multi_line => {
                let x = match self.edit_x_coord {
                    Some(x) => x,
                    None => self
                        .text
                        .text_glyph_pos(cursor)?
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = self.text.find_line(cursor)?.map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                line = match cmd {
                    Command::Up => line.wrapping_sub(1),
                    Command::Down => line.wrapping_add(1),
                    _ => unreachable!(),
                };
                const HALF: usize = usize::MAX / 2;
                let nearest_end = match line {
                    0..=HALF => len,
                    _ => 0,
                };
                self.text
                    .line_index_nearest(line, x)?
                    .map(|index| Action::Move(index, Some(x)))
                    .unwrap_or(Action::Move(nearest_end, None))
            }
            Command::Home if cursor > 0 => {
                let index = self.text.find_line(cursor)?.map(|r| r.1.start).unwrap_or(0);
                Action::Move(index, None)
            }
            Command::End if cursor < len => {
                let index = self.text.find_line(cursor)?.map(|r| r.1.end).unwrap_or(len);
                Action::Move(index, None)
            }
            Command::DocHome if cursor > 0 => Action::Move(0, None),
            Command::DocEnd if cursor < len => Action::Move(len, None),
            // Avoid use of unused navigation keys (e.g. by ScrollComponent):
            Command::Home | Command::End | Command::DocHome | Command::DocEnd => Action::None,
            Command::PageUp | Command::PageDown if multi_line => {
                let mut v = self
                    .text
                    .text_glyph_pos(cursor)?
                    .next_back()
                    .map(|r| r.pos.into())
                    .unwrap_or(Vec2::ZERO);
                if let Some(x) = self.edit_x_coord {
                    v.0 = x;
                }
                const FACTOR: f32 = 2.0 / 3.0;
                let mut h_dist = f32::conv(self.text.rect().size.1) * FACTOR;
                if cmd == Command::PageUp {
                    h_dist *= -1.0;
                }
                v.1 += h_dist;
                Action::Move(self.text.text_index_nearest(v)?, Some(v.0))
            }
            Command::Delete | Command::DelBack if editable && have_sel => {
                Action::Delete(selection.clone(), EditOp::Delete)
            }
            Command::Delete if editable => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.text.text(), 0)
                .unwrap()
                .map(|next| Action::Delete(cursor..next, EditOp::Delete))
                .unwrap_or(Action::None),
            Command::DelBack if editable => {
                // We always delete one code-point, not one grapheme cluster:
                let prev = self.text.text()[0..cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                Action::Delete(prev..cursor, EditOp::Delete)
            }
            Command::DelWord if editable => {
                let next = self.text.text()[cursor..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(len);
                Action::Delete(cursor..next, EditOp::Delete)
            }
            Command::DelWordBack if editable => {
                let prev = self.text.text()[0..cursor]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Delete(prev..cursor, EditOp::Delete)
            }
            Command::SelectAll => {
                self.selection.set_sel_index(0);
                shift = true; // hack
                Action::Move(len, None)
            }
            Command::Cut if editable && have_sel => {
                cx.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::Delete(selection.clone(), EditOp::Clipboard)
            }
            Command::Copy if have_sel => {
                cx.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::None
            }
            Command::Paste if editable => {
                if let Some(content) = cx.get_clipboard() {
                    let range = self.trim_paste(&content);
                    string = content;
                    Action::Insert(&string[range], EditOp::Clipboard)
                } else {
                    Action::None
                }
            }
            Command::Undo | Command::Redo if editable => Action::UndoRedo(cmd == Command::Redo),
            _ => return Ok(Unused),
        };

        // We can receive some commands without key focus as a result of
        // selection focus. Request focus on edit actions (like Command::Cut).
        if !matches!(action, Action::None | Action::Deselect) {
            self.request_key_focus(cx, FocusSource::Synthetic);
        }

        if !matches!(action, Action::None) {
            self.cancel_selection_and_ime(cx);
        }

        let edit_action = match action {
            Action::None => return Ok(Used),
            Action::Deselect | Action::Move(_, _) => Some(EditOp::Cursor),
            Action::Activate | Action::UndoRedo(_) => None,
            Action::Insert(_, edit) | Action::Delete(_, edit) => Some(edit),
        };
        self.save_undo_state(edit_action);

        let mut force_set_offset = false;
        let result = match action {
            Action::None => EditAction::None,
            Action::Deselect => {
                self.selection.set_empty();
                cx.redraw();
                EditAction::None
            }
            Action::Activate => {
                force_set_offset = true;
                EditAction::Activate
            }
            Action::Insert(s, _) => {
                let mut index = cursor;
                let range = if have_sel {
                    index = selection.start;
                    selection.clone()
                } else {
                    index..index
                };
                self.text.replace_range(range, s);
                self.selection.set_cursor(index + s.len());
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Delete(sel, _) => {
                self.text.replace_range(sel.clone(), "");
                self.selection.set_cursor(sel.start);
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Move(index, x_coord) => {
                self.selection.set_edit_index(index);
                if !shift {
                    self.selection.set_empty();
                } else {
                    self.set_primary(cx);
                }
                self.edit_x_coord = x_coord;
                force_set_offset = true;
                cx.redraw();
                EditAction::None
            }
            Action::UndoRedo(redo) => {
                if let Some((text, cursor)) = self.undo_stack.undo_or_redo(redo) {
                    if self.text.set_str(text) {
                        self.edit_x_coord = None;
                        self.error_state = false;
                    }
                    self.selection = (*cursor).into();
                    EditAction::Edit
                } else {
                    return Ok(Used);
                }
            }
        };

        self.prepare_text(cx, force_set_offset);

        Ok(match result {
            EditAction::None => Used,
            EditAction::Activate => {
                cx.depress_with_key(&self, code);
                G::activate(self, cx, data)
            }
            EditAction::Edit => {
                G::edit(self, cx, data);
                Used
            }
        })
    }

    // Set cursor position. It is assumed that the text has not changed.
    fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos = (coord - self.rect().pos).cast();
        if let Ok(index) = self.text.text_index_nearest(rel_pos) {
            if index != self.selection.edit_index() {
                self.selection.set_edit_index(index);
                self.set_view_offset_from_cursor(cx);
                self.edit_x_coord = None;
                cx.redraw();
            }
        }
    }

    fn set_primary(&self, cx: &mut EventCx) {
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
    fn set_view_offset_from_cursor(&mut self, cx: &mut EventCx) {
        let cursor = self.selection.edit_index();
        if let Some(marker) = self
            .text
            .text_glyph_pos(cursor)
            .ok()
            .and_then(|mut m| m.next_back())
        {
            let y0 = (marker.pos.1 - marker.ascent).cast_floor();
            let pos = self.rect().pos + Offset(marker.pos.0.cast_nearest(), y0);
            let size = Size(0, i32::conv_ceil(marker.pos.1 - marker.descent) - y0);
            cx.set_scroll(Scroll::Rect(Rect { pos, size }));
        }
    }
}
