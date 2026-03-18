// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditBox`] widget

use super::*;
use crate::edit::highlight::{Highlighter, Plain};
use crate::{ScrollBar, ScrollBarMsg};
use kas::event::Scroll;
use kas::event::components::ScrollComponent;
use kas::prelude::*;
use kas::theme::{FrameStyle, TextClass};
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[impl_self]
mod EditBox {
    /// A text-edit box
    ///
    /// A single- or multi-line editor for unformatted text.
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
    /// to scale to handle large documents via a single `EditBox` widget.
    ///
    /// ### Messages
    ///
    /// [`kas::messages::SetValueText`] may be used to replace the entire text
    /// and [`kas::messages::ReplaceSelectedText`] may be used to replace
    /// selected text when this widget is not [read-only](Editor::is_read_only).
    /// Both add an item to the undo history and invoke the action handler
    /// [`EditGuard::edit`].
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[autoimpl(Debug where G: trait, H: trait)]
    #[autoimpl(Deref<Target = Editor> using self.inner)]
    #[widget]
    pub struct EditBox<G: EditGuard = DefaultGuard<()>, H: Highlighter = Plain> {
        core: widget_core!(),
        scroll: ScrollComponent,
        // NOTE: inner is a Viewport which doesn't use update methods, therefore we don't call them.
        #[widget]
        inner: EditBoxCore<G, H>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
        frame_style: FrameStyle,
        frame_offset: Offset,
        frame_size: Size,
        frame_offset_ex_margin: Offset,
        inner_margin: i32,
        clip_rect: Rect,
    }

    impl Layout for Self {
        fn size_rules(&mut self, cx: &mut SizeCx, mut axis: AxisInfo) -> SizeRules {
            let size = self.frame_size.extract(axis.flipped());
            axis.map_other(|x| x - size);

            let mut rules = self.inner.size_rules(cx, axis);
            let bar_rules = self.vert_bar.size_rules(cx, axis);
            if axis.is_horizontal() && self.multi_line() {
                self.inner_margin = rules.margins_i32().1.max(bar_rules.margins_i32().0);
                rules.append(bar_rules);
            }

            let frame_rules = cx.frame(self.frame_style, axis);
            self.frame_offset_ex_margin
                .set_component(axis, frame_rules.size());
            let (rules, offset, size) = frame_rules.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut SizeCx, outer_rect: Rect, hints: AlignHints) {
            self.core.set_rect(outer_rect);
            let mut rect = outer_rect;

            self.clip_rect = Rect {
                pos: rect.pos + self.frame_offset_ex_margin,
                size: rect.size - (self.frame_offset_ex_margin * 2).cast(),
            };

            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;

            let mut bar_rect = Rect::ZERO;
            if self.multi_line() {
                // Set bar position, dependent on text direction. TODO: move on text-dir-change.
                let bar_width = cx.scroll_bar_width();
                let (x0, x1);
                if !self.inner.text_is_rtl() {
                    x1 = rect.pos.0 + rect.size.0;
                    x0 = x1 - bar_width;
                } else {
                    x0 = rect.pos.0;
                    x1 = x0 + bar_width;
                    rect.pos.0 = x1;
                }
                bar_rect = Rect::new(Coord(x0, rect.pos.1), Size(bar_width, rect.size.1));
                rect.size.0 = (rect.size.0 - bar_width - self.inner_margin).max(0);
            }
            self.vert_bar.set_rect(cx, bar_rect, AlignHints::NONE);

            self.inner.set_rect(cx, rect, hints);
            self.update_content_size(cx);
        }

        fn draw(&self, mut draw: DrawCx) {
            let mut draw_inner = draw.re();
            draw_inner.set_id(self.inner.id());
            let bg = self.inner.background_color();
            draw_inner.frame(self.rect(), self.frame_style, bg);

            self.inner
                .draw_with_offset(draw.re(), self.clip_rect, self.scroll.offset());

            if self.scroll.max_offset().1 > 0 {
                self.vert_bar.draw(draw.re());
            }
        }
    }

    impl Tile for Self {
        #[inline]
        fn tooltip(&self) -> Option<&str> {
            self.error_message()
        }

        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll.offset(),
                max_offset: self.scroll.max_offset(),
            }
        }

        fn translation(&self, index: usize) -> Offset {
            if index == widget_index!(self.inner) {
                self.scroll.offset()
            } else {
                Offset::ZERO
            }
        }
    }

    impl Events for Self {
        type Data = G::Data;

        fn probe(&self, coord: Coord) -> Id {
            if self.scroll.max_offset().1 > 0 {
                if let Some(id) = self.vert_bar.try_probe(coord) {
                    return id;
                }
            }

            // If coord is over self but not over self.vert_bar, we assign
            // the event to self.inner without further question.
            self.inner.id()
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let rect = Rect {
                pos: self.rect().pos + self.frame_offset,
                size: self.rect().size - self.frame_size,
            };
            let used = self.scroll.scroll_by_event(cx, event, self.id(), rect);
            self.update_content_size(cx);
            used
        }

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, data: &G::Data) {
            let offset = if cx.last_child() == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                Offset(self.scroll.offset().0, y)
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                offset
            } else {
                self.inner.handle_messages(cx, data);
                return;
            };

            if let Some(moved) = self.scroll.set_offset(offset) {
                cx.action_moved(moved);
                self.update_scroll_offset(cx);
            }
        }

        fn handle_resize(&mut self, cx: &mut ConfigCx, _: &Self::Data) -> Option<ActionResize> {
            let size = self.inner.rect().size;
            let axis = AxisInfo::new(false, Some(size.1));
            let mut resize = self.inner.size_rules(&mut cx.size_cx(), axis).min_size() > size.0;
            let axis = AxisInfo::new(true, Some(size.0));
            resize |= self.inner.size_rules(&mut cx.size_cx(), axis).min_size() > size.1;
            self.update_content_size(cx);
            resize.then_some(ActionResize)
        }

        fn handle_scroll(&mut self, cx: &mut EventCx<'_>, _: &G::Data, scroll: Scroll) {
            let rect = self.inner.rect();
            self.scroll.scroll(cx, self.id(), rect, scroll);
            self.update_scroll_offset(cx);
        }
    }

    impl<G: EditGuard> Default for EditBox<G, Plain>
    where
        G: Default,
    {
        #[inline]
        fn default() -> Self {
            EditBox::new(G::default())
        }
    }

    impl<G: EditGuard> EditBox<G, Plain> {
        /// Construct an `EditBox` with an [`EditGuard`]
        #[inline]
        pub fn new(guard: G) -> Self {
            EditBox {
                core: Default::default(),
                scroll: Default::default(),
                inner: EditBoxCore::new(guard),
                vert_bar: Default::default(),
                frame_style: FrameStyle::EditBox,
                frame_offset: Default::default(),
                frame_size: Default::default(),
                frame_offset_ex_margin: Default::default(),
                inner_margin: Default::default(),
                clip_rect: Default::default(),
            }
        }
    }

    impl Self {
        /// Replace the highlighter
        ///
        /// This function reconstructs the text with a new highlighter.
        #[inline]
        pub fn with_highlighter<H2: Highlighter>(self, highlighter: H2) -> EditBox<G, H2> {
            EditBox {
                core: self.core,
                scroll: self.scroll,
                inner: self.inner.with_highlighter(highlighter),
                vert_bar: self.vert_bar,
                frame_style: self.frame_style,
                frame_offset: self.frame_offset,
                frame_size: self.frame_size,
                frame_offset_ex_margin: self.frame_offset_ex_margin,
                inner_margin: self.inner_margin,
                clip_rect: self.clip_rect,
            }
        }

        /// Set a new highlighter of the same type
        pub fn set_highlighter(&mut self, highlighter: H) {
            self.inner.set_highlighter(highlighter);
        }

        /// Replace the frame style
        ///
        /// The default is [`FrameStyle::EditBox`].
        #[inline]
        pub fn with_frame_style(mut self, style: FrameStyle) -> Self {
            self.frame_style = style;
            self
        }

        fn update_content_size(&mut self, cx: &mut EventState) {
            if !self.core.status.is_sized() {
                return;
            }
            let size = self.inner.rect().size;
            let _ = self.scroll.set_sizes(size, self.inner.content_size());
            let max_offset = self.scroll.max_offset().1;
            self.vert_bar.set_limits(cx, max_offset, size.1);
            self.update_scroll_offset(cx);
        }

        fn update_scroll_offset(&mut self, cx: &mut EventState) {
            self.vert_bar.set_value(cx, self.scroll.offset().1);
        }

        /// Access the edit guard
        #[inline]
        pub fn guard(&self) -> &G {
            &self.inner.guard
        }

        /// Access the edit guard mutably
        #[inline]
        pub fn guard_mut(&mut self) -> &mut G {
            &mut self.inner.guard
        }
    }
}

impl<A: 'static> EditBox<DefaultGuard<A>> {
    /// Construct an `EditBox` with the given inital `text` (no event handling)
    #[inline]
    pub fn text<S: ToString>(text: S) -> Self {
        EditBox {
            inner: EditBoxCore::text(text),
            ..Default::default()
        }
    }

    /// Construct a read-only `EditBox` displaying some `String` value
    #[inline]
    pub fn string(value_fn: impl Fn(&A) -> String + Send + 'static) -> EditBox<StringGuard<A>> {
        EditBox::new(StringGuard::new(value_fn)).with_read_only(true)
    }

    /// Construct an `EditBox` for a parsable value (e.g. a number)
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
    ) -> EditBox<ParseGuard<A, T>> {
        EditBox::new(ParseGuard::new(value_fn, msg_fn))
    }

    /// Construct an `EditBox` for a parsable value (e.g. a number)
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
    ) -> EditBox<InstantParseGuard<A, T>> {
        EditBox::new(InstantParseGuard::new(value_fn, msg_fn))
    }
}

impl<A: 'static> EditBox<StringGuard<A>> {
    /// Assign a message function for a `String` value
    ///
    /// The `msg_fn` is called when the field is activated (<kbd>Enter</kbd>)
    /// and when it loses focus after content is changed.
    ///
    /// This method sets self as editable (see [`Self::with_read_only`]).
    #[must_use]
    pub fn with_msg<M>(mut self, msg_fn: impl Fn(&str) -> M + Send + 'static) -> Self
    where
        M: Debug + 'static,
    {
        self.inner.guard = self.inner.guard.with_msg(msg_fn);
        self.inner = self.inner.with_read_only(false);
        self
    }
}

impl<G: EditGuard, H: Highlighter> EditBox<G, H> {
    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `EditBox`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.inner = self.inner.with_text(text);
        self
    }

    /// Set whether this `EditBox` is read-only (inline)
    #[inline]
    #[must_use]
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.inner = self.inner.with_read_only(read_only);
        self
    }

    /// Set whether this `EditBox` uses multi-line mode
    ///
    /// This affects the (vertical) size allocation, alignment, text wrapping
    /// and whether the <kbd>Enter</kbd> key may instert a line break.
    #[inline]
    #[must_use]
    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.inner = self.inner.with_multi_line(multi_line);
        self
    }

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.inner = self.inner.with_class(class);
        self
    }

    /// Adjust the height allocation
    #[inline]
    pub fn set_lines(&mut self, min_lines: f32, ideal_lines: f32) {
        self.inner.set_lines(min_lines, ideal_lines);
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
        self.inner.set_width_em(min_em, ideal_em);
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
        self.inner.edit(cx, data, edit)
    }
}
