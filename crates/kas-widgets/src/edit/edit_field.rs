// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use kas::event::Ime;
use kas::event::components::TextInputAction;
use kas::event::{CursorIcon, ElementState, FocusSource, PhysicalKey};
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::text::{Effect, EffectFlags, NotReady};
use kas::theme::TextClass;
use std::fmt::{Debug, Display};
use std::str::FromStr;

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
    /// [`ReplaceSelectedText`] may be used to replace selected text when this
    /// widget is [editable](Editor::is_editable)]. This triggers the action
    /// handlers [`EditGuard::edit`] followed by [`EditGuard::activate`].
    ///
    /// ### Special behaviour
    ///
    /// This is a [`Viewport`] widget.
    #[autoimpl(Debug where G: trait)]
    #[autoimpl(Deref, DerefMut using self.editor)]
    #[widget]
    pub struct EditField<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        width: (f32, f32),
        lines: (f32, f32),
        editor: Editor,
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
            self.editor.pos = rect.pos;
            hints.vert = Some(if self.multi_line() {
                Align::Default
            } else {
                Align::Center
            });
            self.text.set_rect(cx, rect, hints);
            self.text.ensure_no_left_overhang();
            if self.current.is_ime_enabled() {
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
            self.editor.id = self.id();
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
                        self.call_guard_edit(cx, data);
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
                        self.prepare_and_scroll(cx, false);
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
                        self.prepare_and_scroll(cx, false);
                        self.call_guard_edit(cx, data);
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
                        self.prepare_and_scroll(cx, false);

                        self.call_guard_edit(cx, data);
                    }

                    self.request_key_focus(cx, FocusSource::Pointer);
                    Used
                }
                event => match self.editor.input_handler.handle(cx, self.core.id(), event) {
                    TextInputAction::Used => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::PressStart {
                        coord,
                        clear,
                        repeats,
                    } => {
                        if self.current.is_ime_enabled() {
                            self.clear_ime();
                            cx.cancel_ime_focus(self.id_ref());
                        }
                        self.save_undo_state(Some(EditOp::Cursor));
                        self.current = CurrentAction::Selection;

                        self.set_cursor_from_coord(cx, coord);
                        self.selection.set_anchor(clear);
                        if repeats > 1 {
                            self.editor
                                .selection
                                .expand(&self.editor.text, repeats >= 3);
                        }

                        self.request_key_focus(cx, FocusSource::Pointer);
                        Used
                    }
                    TextInputAction::PressMove { coord, repeats } => {
                        if self.current == CurrentAction::Selection {
                            self.set_cursor_from_coord(cx, coord);
                            if repeats > 1 {
                                self.editor
                                    .selection
                                    .expand(&self.editor.text, repeats >= 3);
                            }
                        }

                        Used
                    }
                    TextInputAction::PressEnd { coord } => {
                        if self.current.is_ime_enabled() {
                            self.clear_ime();
                            cx.cancel_ime_focus(self.id_ref());
                        }
                        self.save_undo_state(Some(EditOp::Cursor));
                        if self.current == CurrentAction::Selection {
                            self.set_primary(cx);
                        } else {
                            self.set_cursor_from_coord(cx, coord);
                            self.selection.set_empty();
                        }
                        self.current = CurrentAction::None;

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
                self.call_guard_edit(cx, data);
            } else if let Some(ReplaceSelectedText(text)) = cx.try_pop() {
                self.pre_commit();
                self.replace_selected_text(cx, &text);
                self.call_guard_edit(cx, data);
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
                width: (8.0, 16.0),
                lines: (1.0, 1.0),
                editor: Editor::new(),
                guard,
            }
        }

        /// Call the [`EditGuard`]'s `activate` method
        #[inline]
        pub fn call_guard_activate(&mut self, cx: &mut EventCx, data: &G::Data) {
            G::activate(self, cx, data);
        }

        /// Call the [`EditGuard`]'s `edit` method
        ///
        /// This call also clears the [error state](Editor::set_error_state).
        #[inline]
        pub fn call_guard_edit(&mut self, cx: &mut EventCx, data: &G::Data) {
            self.set_error_state(cx, false);
            G::edit(self, cx, data);
        }
    }
}

impl<A: 'static> EditField<DefaultGuard<A>> {
    /// Construct an `EditField` with the given inital `text` (no event handling)
    #[inline]
    pub fn text<S: ToString>(text: S) -> Self {
        EditField {
            editor: Editor::from(text),
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

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.text.set_class(class);
        self
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

    fn control_key(
        &mut self,
        cx: &mut EventCx,
        data: &G::Data,
        cmd: Command,
        code: Option<PhysicalKey>,
    ) -> Result<IsUsed, NotReady> {
        let action = self.editor.cmd_action(cx, cmd)?;
        if matches!(action, CmdAction::Unused) {
            return Ok(Unused);
        }

        self.prepare_and_scroll(cx, true);

        Ok(match action {
            CmdAction::Unused => Unused,
            CmdAction::Used | CmdAction::Cursor => Used,
            CmdAction::Activate => {
                cx.depress_with_key(&self, code);
                G::activate(self, cx, data)
            }
            CmdAction::Edit => {
                self.call_guard_edit(cx, data);
                Used
            }
        })
    }
}
