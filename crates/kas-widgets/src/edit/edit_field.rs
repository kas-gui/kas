// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{CursorIcon, ElementState, FocusSource, ImePurpose, PhysicalKey, Scroll};
use kas::geom::Vec2;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::text::{NotReady, SelectionHelper};
use kas::theme::{Text, TextClass};
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
    #[autoimpl(Clone, Debug where G: trait)]
    #[widget]
    pub struct EditField<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        editable: bool,
        width: (f32, f32),
        lines: (f32, f32),
        text: Text<String>,
        selection: SelectionHelper,
        edit_x_coord: Option<f32>,
        old_state: Option<(String, usize, usize)>,
        last_edit: LastEdit,
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

        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, mut ideal): (i32, i32);
            if axis.is_horizontal() {
                let dpem = sizer.dpem();
                min = (self.width.0 * dpem).cast_ceil();
                ideal = (self.width.1 * dpem).cast_ceil();
            } else {
                // TODO: line height depends on the font; 1em is not a good
                // approximation. This code also misses inter-line spacing.
                let dpem = sizer.dpem();
                min = (self.lines.0 * dpem).cast_ceil();
                ideal = (self.lines.1 * dpem).cast_ceil();
            };

            let rules = self.text.size_rules(sizer.re(), axis);
            ideal = ideal.max(rules.ideal_size());

            let margins = sizer.text_margins().extract(axis);
            let stretch = if axis.is_horizontal() || self.multi_line() {
                Stretch::High
            } else {
                Stretch::None
            };
            SizeRules::new(min, ideal, margins, stretch)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect, mut hints: AlignHints) {
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

        fn draw(&self, draw: DrawCx) {
            self.draw_with_offset(draw, self.rect(), Offset::ZERO);
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
                cursor: self.selection.edit_index(),
                sel_index: self.selection.sel_index(),
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
            cx.text_configure(&mut self.text);
            G::configure(self, cx);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &G::Data) {
            G::update(self, cx, data);
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &G::Data, event: Event) -> IsUsed {
            match event {
                Event::NavFocus(source) if source == FocusSource::Key => {
                    if !self.has_key_focus && !self.current.is_select() {
                        let ime = Some(ImePurpose::Normal);
                        cx.request_key_focus(self.id(), ime, source);
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
                    Used
                }
                Event::ImeFocus => {
                    self.current = CurrentAction::ImeStart;
                    self.set_ime_cursor_area(cx);
                    Used
                }
                Event::LostImeFocus => {
                    if self.current.is_ime() {
                        self.current = CurrentAction::None;
                    }
                    Used
                }
                Event::LostKeyFocus => {
                    self.has_key_focus = false;
                    cx.redraw(&self);
                    G::focus_lost(self, cx, data);
                    Used
                }
                Event::LostSelFocus => {
                    // IME focus without selection focus is impossible, so we can clear all current actions
                    self.current = CurrentAction::None;
                    self.selection.set_empty();
                    cx.redraw(self);
                    Used
                }
                Event::Command(cmd, code) => match self.control_key(cx, data, cmd, code) {
                    Ok(r) => r,
                    Err(NotReady) => Used,
                },
                Event::Key(event, false) if event.state == ElementState::Pressed => {
                    if let Some(text) = &event.text {
                        let used = self.received_text(cx, text);
                        G::edit(self, cx, data);
                        used
                    } else {
                        let opt_cmd = cx
                            .config()
                            .shortcuts()
                            .try_match(cx.modifiers(), &event.logical_key);
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
                Event::ImePreedit(text, cursor) => {
                    if self.current != CurrentAction::ImeEdit {
                        if cursor.is_some() {
                            self.selection.set_anchor_to_range_start();
                            self.current = CurrentAction::ImeEdit;
                        } else {
                            return Used;
                        }
                    }

                    let range = self.selection.anchor_to_edit_range();
                    self.text.replace_range(range.clone(), text);

                    if let Some((start, end)) = cursor {
                        self.selection.set_sel_index_only(range.start + start);
                        self.selection.set_edit_index(range.start + end);
                    } else {
                        self.selection.set_all(range.start + text.len());
                    }
                    self.edit_x_coord = None;
                    self.prepare_text(cx);
                    Used
                }
                Event::ImeCommit(text) => {
                    if self.current != CurrentAction::ImeEdit {
                        self.selection.set_anchor_to_range_start();
                    }
                    self.current = CurrentAction::None;

                    let range = self.selection.anchor_to_edit_range();
                    self.text.replace_range(range.clone(), text);

                    self.selection.set_all(range.start + text.len());
                    self.edit_x_coord = None;
                    self.prepare_text(cx);
                    Used
                }
                Event::PressStart(press) if press.is_tertiary() => {
                    press.grab_click(self.id()).complete(cx)
                }
                Event::PressEnd { press, .. } if press.is_tertiary() => {
                    if let Some(content) = cx.get_primary() {
                        self.set_cursor_from_coord(cx, press.coord);
                        self.current.clear_selection();
                        self.selection.set_empty();
                        let index = self.selection.edit_index();
                        let range = self.trim_paste(&content);
                        let len = range.len();

                        self.old_state =
                            Some((self.text.clone_string(), index, self.selection.sel_index()));
                        self.last_edit = LastEdit::Paste;

                        self.text.replace_range(index..index, &content[range]);
                        self.selection.set_all(index + len);
                        self.edit_x_coord = None;
                        self.prepare_text(cx);

                        G::edit(self, cx, data);
                    }
                    Used
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::Used => Used,
                    TextInputAction::Unused => Unused,
                    TextInputAction::CursorStart {
                        coord,
                        clear,
                        repeats,
                    } => {
                        if self.current.is_ime() {
                            cx.cancel_ime_focus(self.id());
                        }
                        self.set_cursor_from_coord(cx, coord);
                        self.selection.set_anchor(clear);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }

                        if !self.has_key_focus {
                            cx.request_key_focus(self.id(), None, FocusSource::Pointer);
                        }
                        self.current = CurrentAction::DragSelect;
                        Used
                    }
                    TextInputAction::CursorMove { coord, repeats } if self.current.is_select() => {
                        self.set_cursor_from_coord(cx, coord);
                        if repeats > 1 {
                            self.selection.expand(&self.text, repeats >= 3);
                        }

                        Used
                    }
                    TextInputAction::CursorEnd { .. } if self.current.is_select() => {
                        self.current = CurrentAction::None;
                        self.set_primary(cx);
                        let ime = Some(ImePurpose::Normal);
                        cx.request_key_focus(self.id(), ime, FocusSource::Pointer);
                        Used
                    }
                    _ => Used,
                },
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &G::Data) {
            if !self.editable {
                return;
            }

            if let Some(SetValueText(string)) = cx.try_pop() {
                self.set_string(cx, string);
                G::edit(self, cx, data);
                G::activate(self, cx, data);
            } else if let Some(ReplaceSelectedText(text)) = cx.try_pop() {
                self.received_text(cx, &text);
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
                text: Text::default().with_class(TextClass::Edit(false)),
                selection: Default::default(),
                edit_x_coord: None,
                old_state: None,
                last_edit: Default::default(),
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

        /// Set text contents from a `str`
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
        /// This method does not call action handlers on the [`EditGuard`].
        ///
        /// Returns `true` if the text is ready and may have changed.
        pub fn set_string(&mut self, cx: &mut EventState, string: String) -> bool {
            if !self.text.set_string(string) || !self.text.prepare() {
                return false;
            }

            self.current.clear_active();
            self.selection.set_max_len(self.text.str_len());
            cx.redraw(&self);
            if self.current.is_ime() {
                self.set_ime_cursor_area(cx);
            }
            self.set_error_state(cx, false);
            true
        }

        /// Replace selected text
        ///
        /// This method does not call action handlers on the [`EditGuard`].
        pub fn replace_selection(&mut self, cx: &mut EventCx, text: &str) {
            self.received_text(cx, text);
        }

        // Call only if self.ime_focus
        fn set_ime_cursor_area(&self, cx: &mut EventState) {
            if let Ok(display) = self.text.display() {
                if let Some(mut rect) = self.selection.cursor_rect(display) {
                    rect.pos += Offset::conv(self.rect().pos);
                    cx.set_ime_cursor_area(self.id_ref(), rect);
                }
            }
        }

        /// Get the size of the type-set text
        ///
        /// `EditField` ensures text has no left or top overhang.
        #[inline]
        pub fn typeset_size(&self) -> Size {
            let mut size = self.rect().size;
            if let Ok((tl, br)) = self.text.bounding_box() {
                size.1 = size.1.max((br.1 - tl.1).cast_ceil());
                size.0 = size.0.max((br.0 - tl.0).cast_ceil());
            }
            size
        }

        /// Draw with an offset
        ///
        /// Draws at position `self.rect() - offset`.
        ///
        /// This may be called instead of [`Layout::draw`].
        pub fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            let pos = self.rect().pos - offset;

            draw.text_selected(pos, rect, &self.text, self.selection.range());

            if self.editable && draw.ev_state().has_key_focus(self.id_ref()).0 {
                draw.text_cursor(pos, rect, &self.text, self.selection.edit_index());
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
            text: Text::new(text, TextClass::Edit(false)),
            selection: SelectionHelper::new(len, len),
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
        debug_assert!(self.current == CurrentAction::None);
        let text = text.to_string();
        let len = text.len();
        self.text.set_string(text);
        self.selection.set_all(len);
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
        self.text.set_class(TextClass::Edit(multi_line));
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
        self.class().multi_line()
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

    fn prepare_text(&mut self, cx: &mut EventCx) {
        if self.text.prepare() {
            self.text.ensure_no_left_overhang();
            cx.redraw(&self);
        }

        self.set_view_offset_from_cursor(cx);
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
        if !self.editable || self.current.is_active_ime() {
            return Unused;
        }

        self.current.clear_selection();
        let index = self.selection.edit_index();
        let selection = self.selection.range();
        let have_sel = selection.start < selection.end;
        if self.last_edit != LastEdit::Insert || have_sel {
            self.old_state = Some((self.text.clone_string(), index, self.selection.sel_index()));
            self.last_edit = LastEdit::Insert;
        }
        if have_sel {
            self.text.replace_range(selection.clone(), text);
            self.selection.set_all(selection.start + text.len());
        } else {
            // TODO(kas-text) support the following:
            // self.text.insert_str(index, text);
            let mut s = self.text.clone_string();
            s.insert_str(index, text);
            self.text.set_text(s);
            // END workaround
            self.selection.set_all(index + text.len());
        }
        self.edit_x_coord = None;

        self.prepare_text(cx);
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
            Activate,
            Edit,
            Insert(&'a str, LastEdit),
            Delete(Range<usize>),
            Move(usize, Option<f32>),
        }

        let action = match cmd {
            Command::Escape | Command::Deselect
                if !self.current.is_active_ime() && !selection.is_empty() =>
            {
                self.current.clear_selection();
                self.selection.set_empty();
                cx.redraw(&self);
                Action::None
            }
            Command::Activate => Action::Activate,
            Command::Enter if shift || !multi_line => Action::Activate,
            Command::Enter if editable && multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), LastEdit::Insert)
            }
            // NOTE: we might choose to optionally handle Tab in the future,
            // but without some workaround it prevents keyboard navigation.
            // Command::Tab => Action::Insert('\t'.encode_utf8(&mut buf), LastEdit::Insert),
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
                Action::Delete(selection.clone())
            }
            Command::Delete if editable => GraphemeCursor::new(cursor, len, true)
                .next_boundary(self.text.text(), 0)
                .unwrap()
                .map(|next| Action::Delete(cursor..next))
                .unwrap_or(Action::None),
            Command::DelBack if editable => {
                // We always delete one code-point, not one grapheme cluster:
                let prev = self.text.text()[0..cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                Action::Delete(prev..cursor)
            }
            Command::DelWord if editable => {
                let next = self.text.text()[cursor..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| cursor + index)
                    .unwrap_or(len);
                Action::Delete(cursor..next)
            }
            Command::DelWordBack if editable => {
                let prev = self.text.text()[0..cursor]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Delete(prev..cursor)
            }
            Command::SelectAll => {
                self.selection.set_sel_index(0);
                shift = true; // hack
                Action::Move(len, None)
            }
            Command::Cut if editable && have_sel => {
                cx.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::Delete(selection.clone())
            }
            Command::Copy if have_sel => {
                cx.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::None
            }
            Command::Paste if editable => {
                if let Some(content) = cx.get_clipboard() {
                    let range = self.trim_paste(&content);
                    string = content;
                    Action::Insert(&string[range], LastEdit::Paste)
                } else {
                    Action::None
                }
            }
            Command::Undo | Command::Redo if editable => {
                // TODO: maintain full edit history (externally?)
                if let Some((state, c2, sel)) = self.old_state.as_mut() {
                    self.text.swap_string(state);
                    self.selection.set_edit_index(*c2);
                    *c2 = cursor;
                    let index = *sel;
                    *sel = self.selection.sel_index();
                    self.selection.set_sel_index(index);
                    self.edit_x_coord = None;
                    self.last_edit = LastEdit::None;
                }
                Action::Edit
            }
            _ => return Ok(Unused),
        };

        if !self.has_key_focus {
            // This can happen if we still had selection focus, then received
            // e.g. Command::Copy.
            let ime = Some(ImePurpose::Normal);
            cx.request_key_focus(self.id(), ime, FocusSource::Synthetic);
        }

        if !matches!(action, Action::None) {
            self.current = CurrentAction::None;
        }

        let result = match action {
            Action::None => EditAction::None,
            Action::Activate => EditAction::Activate,
            Action::Edit => EditAction::Edit,
            Action::Insert(s, edit) => {
                let mut index = cursor;
                if have_sel {
                    self.old_state =
                        Some((self.text.clone_string(), index, self.selection.sel_index()));
                    self.last_edit = edit;

                    self.text.replace_range(selection.clone(), s);
                    index = selection.start;
                } else {
                    if self.last_edit != edit {
                        self.old_state =
                            Some((self.text.clone_string(), index, self.selection.sel_index()));
                        self.last_edit = edit;
                    }

                    self.text.replace_range(index..index, s);
                }
                self.selection.set_all(index + s.len());
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Delete(sel) => {
                if self.last_edit != LastEdit::Delete {
                    self.old_state =
                        Some((self.text.clone_string(), cursor, self.selection.sel_index()));
                    self.last_edit = LastEdit::Delete;
                }

                self.text.replace_range(sel.clone(), "");
                self.selection.set_all(sel.start);
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
                cx.redraw(&self);
                EditAction::None
            }
        };

        self.prepare_text(cx);

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

    fn set_cursor_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos = (coord - self.rect().pos).cast();
        if let Ok(index) = self.text.text_index_nearest(rel_pos) {
            if index != self.selection.edit_index() {
                self.selection.set_edit_index(index);
                self.set_view_offset_from_cursor(cx);
                self.edit_x_coord = None;
                cx.redraw(self);
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
