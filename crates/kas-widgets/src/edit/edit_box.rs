// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use crate::{ScrollBar, ScrollBarMsg};
use kas::event::Scroll;
use kas::event::components::ScrollComponent;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::theme::{Background, FrameStyle, TextClass};
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[impl_self]
mod EditBox {
    /// A text-edit box
    ///
    /// A single- or multi-line editor for unformatted text.
    /// See also notes on [`EditField`].
    ///
    /// By default, the editor supports a single-line only;
    /// [`Self::with_multi_line`] can be used to change this.
    ///
    /// ### Messages
    ///
    /// [`SetValueText`] may be used to replace the entire text and
    /// [`ReplaceSelectedText`] may be used to replace selected text when this
    /// widget is [editable](Editor::is_editable). This triggers the action
    /// handlers [`EditGuard::edit`] followed by [`EditGuard::activate`].
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[autoimpl(Default, Debug where G: trait)]
    #[autoimpl(Deref<Target = Editor>, DerefMut using self.inner)]
    #[widget]
    pub struct EditBox<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        scroll: ScrollComponent,
        // NOTE: inner is a Viewport which doesn't use update methods, therefore we don't call them.
        #[widget]
        inner: EditField<G>,
        #[widget(&())]
        vert_bar: ScrollBar<kas::dir::Down>,
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

            let frame_rules = cx.frame(FrameStyle::EditBox, axis);
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
                let bar_width = cx.scroll_bar_width();
                let x1 = rect.pos.0 + rect.size.0;
                let x0 = x1 - bar_width;
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
            let bg = if self.inner.has_error() {
                Background::Error
            } else {
                Background::Default
            };
            draw_inner.frame(self.rect(), FrameStyle::EditBox, bg);

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
            let action = if cx.last_child() == Some(widget_index![self.vert_bar])
                && let Some(ScrollBarMsg(y)) = cx.try_pop()
            {
                let offset = Offset(self.scroll.offset().0, y);
                self.scroll.set_offset(offset)
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.scroll.set_offset(offset)
            } else if self.is_editable()
                && let Some(SetValueText(string)) = cx.try_pop()
            {
                self.pre_commit();
                self.set_string(cx, string);
                self.inner.call_guard_edit(cx, data);
                return;
            } else if let Some(&ReplaceSelectedText(_)) = cx.try_peek() {
                self.inner.handle_messages(cx, data);
                return;
            } else {
                return;
            };

            if let Some(moved) = action {
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

    impl Self {
        /// Construct an `EditBox` with an [`EditGuard`]
        #[inline]
        pub fn new(guard: G) -> Self {
            EditBox {
                core: Default::default(),
                scroll: Default::default(),
                inner: EditField::new(guard),
                vert_bar: Default::default(),
                frame_offset: Default::default(),
                frame_size: Default::default(),
                frame_offset_ex_margin: Default::default(),
                inner_margin: Default::default(),
                clip_rect: Default::default(),
            }
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

        /// Clear text contents and undo history
        #[inline]
        pub fn clear(&mut self, cx: &mut EventState) {
            self.inner.clear(cx);
        }

        /// Commit outstanding changes to the undo history
        ///
        /// Call this *before* changing the text with `set_str` or `set_string`
        /// to commit changes to the undo history.
        #[inline]
        pub fn pre_commit(&mut self) {
            self.inner.pre_commit();
        }

        // Set text contents from a `str`
        ///
        /// This does not interact with undo history; see also [`Self::clear`],
        /// [`Self::pre_commit`].
        #[inline]
        pub fn set_str(&mut self, cx: &mut EventState, text: &str) {
            if self.inner.set_str(cx, text) {
                self.update_content_size(cx);
            }
        }

        /// Set text contents from a `String`
        ///
        /// This does not interact with undo history; see also [`Self::clear`],
        /// [`Self::pre_commit`].
        ///
        /// This method does not call action handlers on the [`EditGuard`].
        #[inline]
        pub fn set_string(&mut self, cx: &mut EventState, text: String) {
            if self.inner.set_string(cx, text) {
                self.update_content_size(cx);
            }
        }

        /// Replace selected text
        ///
        /// This does not interact with undo history or call action handlers on the
        /// guard.
        #[inline]
        pub fn replace_selected_text(&mut self, cx: &mut EventState, text: &str) {
            if self.inner.replace_selected_text(cx, text) {
                self.update_content_size(cx);
            }
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
            inner: EditField::text(text),
            ..Default::default()
        }
    }

    /// Construct a read-only `EditBox` displaying some `String` value
    #[inline]
    pub fn string(value_fn: impl Fn(&A) -> String + Send + 'static) -> EditBox<StringGuard<A>> {
        EditBox::new(StringGuard::new(value_fn)).with_editable(false)
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
    /// This method sets self as editable (see [`Self::with_editable`]).
    #[must_use]
    pub fn with_msg<M>(mut self, msg_fn: impl Fn(&str) -> M + Send + 'static) -> Self
    where
        M: Debug + 'static,
    {
        self.inner.guard = self.inner.guard.with_msg(msg_fn);
        self.inner.set_editable(true);
        self
    }
}

impl<G: EditGuard> EditBox<G> {
    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `EditBox`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.inner = self.inner.with_text(text);
        self
    }

    /// Set whether this widget is editable (inline)
    #[inline]
    #[must_use]
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.inner = self.inner.with_editable(editable);
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
}
