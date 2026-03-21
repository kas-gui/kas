// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditBoxCore`] widget

use super::editor::{Component, EventAction};
use super::*;
use crate::edit::highlight::{Highlighter, Plain};
use kas::event::CursorIcon;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::theme::{Background, TextClass};
use std::ops::Deref;

#[impl_self]
mod EditBoxCore {
    /// A text-edit field (single- or multi-line)
    ///
    /// The [`EditBox`] widget should be preferred in almost all cases; this
    /// widget is a component of [`EditBox`] and has some special behaviour.
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
    /// to scale to handle large documents via a single `EditBoxCore` widget.
    ///
    /// ### Messages
    ///
    /// [`SetValueText`] may be used to replace the entire text and
    /// [`ReplaceSelectedText`] may be used to replace selected text when this
    /// widget is not [read-only](Editor::is_read_only). Both add an item to
    /// the undo history and invoke the action handler [`EditGuard::edit`].
    ///
    /// ### Special behaviour
    ///
    /// This is a [`Viewport`] widget.
    #[autoimpl(Debug where G: trait, H: trait)]
    #[widget]
    #[layout(self.editor)]
    pub struct EditBoxCore<G: EditGuard = DefaultGuard<()>, H: Highlighter = Plain> {
        core: widget_core!(),
        width: (f32, f32),
        lines: (f32, f32),
        editor: Component<H>,
        /// The associated [`EditGuard`] implementation
        pub guard: G,
    }

    impl Deref for Self {
        type Target = Editor;
        fn deref(&self) -> &Self::Target {
            &self.editor.0
        }
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, mut ideal): (i32, i32);
            if axis.is_horizontal() {
                let dpem = cx.dpem(self.class());
                min = (self.width.0 * dpem).cast_ceil();
                ideal = (self.width.1 * dpem).cast_ceil();
            } else if let Some(width) = axis.other() {
                // Use the height of the first line as a reference
                let height = self
                    .editor
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
            self.editor.part().content_size()
        }

        #[inline]
        fn draw_with_offset(&self, mut draw: DrawCx, rect: Rect, offset: Offset) {
            self.editor.part().draw_with_offset(draw, rect, offset);
        }
    }

    impl Tile for Self {
        fn navigable(&self) -> bool {
            true
        }

        #[inline]
        fn tooltip(&self) -> Option<&str> {
            self.editor.0.error_message()
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::TextInput {
                text: self.as_str(),
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
            self.guard.configure(&mut self.editor.0, cx);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &G::Data) {
            if !self.has_input_focus() {
                self.guard.update(&mut self.editor.0, cx, data);
            }

            self.editor.prepare(cx);
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &G::Data, event: Event) -> IsUsed {
            match self.editor.handle_event(cx, event) {
                EventAction::Unused => Unused,
                EventAction::Used | EventAction::Cursor | EventAction::Preedit => Used,
                EventAction::FocusGained => {
                    self.guard.focus_gained(&mut self.editor.0, cx, data);
                    self.editor.prepare(cx);
                    Used
                }
                EventAction::FocusLost => {
                    self.guard.focus_lost(&mut self.editor.0, cx, data);
                    self.editor.prepare(cx);
                    Used
                }
                EventAction::Activate(code) => {
                    cx.depress_with_key(&self, code);
                    let result = self.guard.activate(&mut self.editor.0, cx, data);
                    self.editor.prepare(cx);
                    result
                }
                EventAction::Edit => {
                    self.call_guard_edit(cx, data);
                    Used
                }
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, data: &G::Data) {
            if self.is_read_only() {
                return;
            }

            if let Some(SetValueText(string)) = cx.try_pop() {
                self.edit(cx, data, |edit, cx| {
                    edit.pre_commit();
                    edit.set_string(cx, string);
                });
            } else if let Some(ReplaceSelectedText(text)) = cx.try_pop() {
                self.edit(cx, data, |edit, cx| {
                    edit.pre_commit();
                    edit.replace_selected_text(cx, &text);
                });
            }
        }
    }

    impl<G: EditGuard> Default for EditBoxCore<G, Plain>
    where
        G: Default,
    {
        #[inline]
        fn default() -> Self {
            EditBoxCore::new(G::default())
        }
    }

    impl<G: EditGuard> EditBoxCore<G, Plain> {
        /// Construct an `EditBox` with an [`EditGuard`]
        #[inline]
        pub fn new(guard: G) -> EditBoxCore<G> {
            EditBoxCore {
                core: Default::default(),
                width: (8.0, 16.0),
                lines: (1.0, 1.0),
                editor: Component::default(),
                guard,
            }
        }
    }

    impl Self {
        /// Replace the highlighter
        ///
        /// This function reconstructs the text with a new highlighter.
        #[inline]
        pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> EditBoxCore<G, H2> {
            EditBoxCore {
                core: self.core,
                width: self.width,
                lines: self.lines,
                editor: self.editor.with_highlighter(highlighter),
                guard: self.guard,
            }
        }

        /// Set a new highlighter of the same type
        #[inline]
        pub fn set_highlighter(&mut self, highlighter: H) {
            self.editor.set_highlighter(highlighter);
        }

        /// Get the background color
        #[inline]
        pub fn background_color(&self) -> Background {
            self.editor.background_color()
        }

        /// Call the [`EditGuard`]'s `activate` method
        #[inline]
        pub fn call_guard_activate(&mut self, cx: &mut EventCx, data: &G::Data) {
            self.guard.activate(&mut self.editor.0, cx, data);
            self.editor.prepare(cx);
        }

        /// Call the [`EditGuard`]'s `edit` method
        ///
        /// This call also clears the error state (see [`Editor::set_error`]).
        #[inline]
        fn call_guard_edit(&mut self, cx: &mut EventCx, data: &G::Data) {
            self.editor.clear_error();
            self.guard.edit(&mut self.editor.0, cx, data);
            self.editor.prepare(cx);
        }
    }
}

impl<A: 'static> EditBoxCore<DefaultGuard<A>> {
    /// Construct an `EditBoxCore` with the given inital `text` (no event handling)
    #[inline]
    pub fn text<S: ToString>(text: S) -> Self {
        EditBoxCore {
            editor: Component::from(text),
            ..Default::default()
        }
    }
}

impl<G: EditGuard, H: Highlighter> EditBoxCore<G, H> {
    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `EditBoxCore`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.editor = self.editor.with_text(text);
        self
    }

    /// Set whether this `EditBoxCore` is read-only (inline)
    #[inline]
    #[must_use]
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.editor.0.set_read_only(read_only);
        self
    }

    /// Set whether this `EditBoxCore` uses multi-line mode
    ///
    /// This affects the (vertical) size allocation, alignment, text wrapping
    /// and whether the <kbd>Enter</kbd> key may instert a line break.
    #[inline]
    #[must_use]
    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.editor.set_wrap(multi_line);
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
        self.editor.set_class(class);
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

    /// Edit text contents
    ///
    /// This method calls the `edit` closure, then [`EditGuard::edit`], then
    /// returns the result of calling `edit`.
    pub fn edit<T>(
        &mut self,
        cx: &mut EventCx,
        data: &G::Data,
        edit: impl FnOnce(&mut Editor, &mut EventCx) -> T,
    ) -> T {
        let result = edit(&mut self.editor.0, cx);
        self.call_guard_edit(cx, data);
        result
    }
}
