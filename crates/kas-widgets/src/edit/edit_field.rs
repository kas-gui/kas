// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use kas::event::CursorIcon;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
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
    /// [`Self::with_multi_line`] can be used to change this.
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
    #[autoimpl(Deref<Target = Editor>, DerefMut using self.editor)]
    #[widget]
    #[layout(self.editor)]
    pub struct EditField<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        width: (f32, f32),
        lines: (f32, f32),
        editor: Component,
        /// The associated [`EditGuard`] implementation
        pub guard: G,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, mut ideal): (i32, i32);
            if axis.is_horizontal() {
                let dpem = cx.dpem(self.text().class());
                min = (self.width.0 * dpem).cast_ceil();
                ideal = (self.width.1 * dpem).cast_ceil();
            } else if let Some(width) = axis.other() {
                // Use the height of the first line as a reference
                let height = self
                    .editor
                    .text_mut()
                    .measure_height(width.cast(), std::num::NonZero::new(1));
                min = (self.lines.0 * height).cast_ceil();
                ideal = (self.lines.1 * height).cast_ceil();
            } else {
                unreachable!()
            };

            let rules = self.editor.size_rules(cx, axis);
            ideal = ideal.max(rules.ideal_size());

            let stretch = if axis.is_horizontal() || self.multi_line() {
                Stretch::High
            } else {
                Stretch::None
            };
            SizeRules::new(min, ideal, stretch).with_margins(cx.text_margins().extract(axis))
        }

        #[inline]
        fn set_rect(&mut self, cx: &mut SizeCx, rect: Rect, mut hints: AlignHints) {
            hints.vert = Some(if self.multi_line() {
                Align::Default
            } else {
                Align::Center
            });
            self.editor.set_rect(cx, rect, hints);
        }
    }

    impl Viewport for Self {
        #[inline]
        fn content_size(&self) -> Size {
            self.editor.content_size()
        }

        #[inline]
        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            self.editor.draw_with_offset(draw, rect, offset);
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        #[inline]
        fn tooltip(&self) -> Option<&str> {
            self.editor.error_message()
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::TextInput {
                text: self.text().as_str(),
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
            self.editor.configure(cx, self.id());
            self.guard.configure(&mut self.editor, cx);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &G::Data) {
            let size = self.content_size();
            if !self.has_input_focus() {
                self.guard.update(&mut self.editor, cx, data);
            }
            if size != self.content_size() {
                cx.resize();
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &G::Data, event: Event) -> IsUsed {
            match self.editor.handle_event(cx, event) {
                EventAction::Unused => Unused,
                EventAction::Used | EventAction::Cursor => Used,
                EventAction::FocusGained => {
                    self.guard.focus_gained(&mut self.editor, cx, data);
                    Used
                }
                EventAction::FocusLost => {
                    self.guard.focus_lost(&mut self.editor, cx, data);
                    Used
                }
                EventAction::Activate(code) => {
                    cx.depress_with_key(&self, code);
                    self.guard.activate(&mut self.editor, cx, data)
                }
                EventAction::Edit => {
                    self.call_guard_edit(cx, data);
                    Used
                }
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &G::Data) {
            if !self.is_editable() {
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
                editor: Component::default(),
                guard,
            }
        }

        /// Call the [`EditGuard`]'s `activate` method
        #[inline]
        pub fn call_guard_activate(&mut self, cx: &mut EventCx, data: &G::Data) {
            self.guard.activate(&mut self.editor, cx, data);
        }

        /// Call the [`EditGuard`]'s `edit` method
        ///
        /// This call also clears the error state (see [`Editor::set_error`]).
        #[inline]
        pub fn call_guard_edit(&mut self, cx: &mut EventCx, data: &G::Data) {
            self.clear_error();
            self.guard.edit(&mut self.editor, cx, data);
        }
    }
}

impl<A: 'static> EditField<DefaultGuard<A>> {
    /// Construct an `EditField` with the given inital `text` (no event handling)
    #[inline]
    pub fn text<S: ToString>(text: S) -> Self {
        EditField {
            editor: Component::from(text),
            ..Default::default()
        }
    }

    /// Construct a read-only `EditField` displaying some `String` value
    #[inline]
    pub fn string(value_fn: impl Fn(&A) -> String + Send + 'static) -> EditField<StringGuard<A>> {
        let mut field = EditField::new(StringGuard::new(value_fn));
        field.set_editable(false);
        field
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
        value_fn: impl Fn(&A) -> T + Send + 'static,
        msg_fn: impl Fn(T) -> M + Send + 'static,
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
        value_fn: impl Fn(&A) -> T + Send + 'static,
        msg_fn: impl Fn(T) -> M + Send + 'static,
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
    pub fn with_msg<M>(mut self, msg_fn: impl Fn(&str) -> M + Send + 'static) -> Self
    where
        M: Debug + 'static,
    {
        self.guard = self.guard.with_msg(msg_fn);
        self.set_editable(true);
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
        self.editor = self.editor.with_text(text);
        self
    }

    /// Set whether this `EditField` is editable (inline)
    #[inline]
    #[must_use]
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.set_editable(editable);
        self
    }

    /// Set whether this `EditField` uses multi-line mode
    ///
    /// This affects the (vertical) size allocation, alignment, text wrapping
    /// and whether the <kbd>Enter</kbd> key may instert a line break.
    #[inline]
    #[must_use]
    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.editor.text_mut().set_wrap(multi_line);
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
        self.editor.text_mut().set_class(class);
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
}
