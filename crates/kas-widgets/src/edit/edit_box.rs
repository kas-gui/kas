// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use super::*;
use crate::{ScrollBar, ScrollMsg};
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
    /// [`Self::with_multi_line`] and [`Self::with_class`] can be used to change this.
    ///
    /// ### Messages
    ///
    /// [`SetValueText`] may be used to replace the entire text and
    /// [`ReplaceSelectedText`] may be used to replace selected text, where
    /// [`Self::is_editable`]. This triggers the action handlers
    /// [`EditGuard::edit`] followed by [`EditGuard::activate`].
    ///
    /// [`kas::messages::SetScrollOffset`] may be used to set the scroll offset.
    #[autoimpl(Clone, Default, Debug where G: trait)]
    #[widget]
    pub struct EditBox<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        scroll: ScrollComponent,
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
            axis.sub_other(self.frame_size.extract(axis.flipped()));

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
            widget_set_rect!(outer_rect);
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
            let _ = self.scroll.set_sizes(rect.size, self.inner.typeset_size());
            self.update_scroll_bar(cx);
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
        fn role(&self, _: &mut dyn RoleCx) -> Role<'_> {
            Role::ScrollRegion {
                offset: self.scroll_offset(),
                max_offset: self.max_scroll_offset(),
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
            self.update_scroll_bar(cx);
            used
        }

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, data: &G::Data) {
            if cx.last_child() == Some(widget_index![self.vert_bar])
                && let Some(ScrollMsg(y)) = cx.try_pop()
            {
                let offset = Offset(self.scroll.offset().0, y);
                let action = self.scroll.set_offset(offset);
                cx.action(&self, action);
                self.update_scroll_bar(cx);
            } else if self.is_editable()
                && let Some(SetValueText(string)) = cx.try_pop()
            {
                self.set_string(cx, string);
                G::edit(&mut self.inner, cx, data);
                G::activate(&mut self.inner, cx, data);
            } else if let Some(kas::messages::SetScrollOffset(offset)) = cx.try_pop() {
                self.set_scroll_offset(cx, offset);
            }
            if let Some(&ReplaceSelectedText(_)) = cx.try_peek() {
                self.inner.handle_messages(cx, data);
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx<'_>, _: &G::Data, scroll: Scroll) {
            // Inner may have resized itself, hence we update sizes now.
            let pos = self.rect().pos + self.frame_offset;
            let size = self.update_content_size();
            let rect = Rect { pos, size };
            self.scroll.scroll(cx, self.id(), rect, scroll);
            self.update_scroll_bar(cx);
        }
    }

    impl Scrollable for Self {
        fn content_size(&self) -> Size {
            self.inner.rect().size
        }

        fn max_scroll_offset(&self) -> Offset {
            self.scroll.max_offset()
        }

        fn scroll_offset(&self) -> Offset {
            self.scroll.offset()
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let action = self.scroll.set_offset(offset);
            let offset = self.scroll.offset();
            if !action.is_empty() {
                cx.action(&self, action);
                self.vert_bar.set_value(cx, offset.1);
            }
            offset
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

        fn update_content_size(&mut self) -> Size {
            let size = self.rect().size - self.frame_size;
            let _ = self.scroll.set_sizes(size, self.inner.typeset_size());
            size
        }

        fn update_scroll_bar(&mut self, cx: &mut EventState) {
            let max_offset = self.scroll.max_offset().1;
            self.vert_bar
                .set_limits(cx, max_offset, self.inner.rect().size.1);
            self.vert_bar.set_value(cx, self.scroll.offset().1);
        }

        /// Get text contents
        #[inline]
        pub fn as_str(&self) -> &str {
            self.inner.as_str()
        }

        /// Get the text contents as a `String`
        #[inline]
        pub fn clone_string(&self) -> String {
            self.inner.clone_string()
        }

        // Set text contents from a `str`
        #[inline]
        pub fn set_str(&mut self, cx: &mut EventState, text: &str) {
            if self.inner.set_str(cx, text) {
                self.update_content_size();
                self.update_scroll_bar(cx);
            }
        }

        /// Set text contents from a `String`
        ///
        /// This method does not call action handlers on the [`EditGuard`].
        #[inline]
        pub fn set_string(&mut self, cx: &mut EventState, text: String) {
            if self.inner.set_string(cx, text) {
                self.update_content_size();
                self.update_scroll_bar(cx);
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
    pub fn string(value_fn: impl Fn(&A) -> String + 'static) -> EditBox<StringGuard<A>> {
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
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
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
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
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
    pub fn with_msg<M>(mut self, msg_fn: impl Fn(&str) -> M + 'static) -> Self
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

    /// Get whether this `EditField` is editable
    #[inline]
    pub fn is_editable(&self) -> bool {
        self.inner.is_editable()
    }

    /// Set whether this `EditField` is editable
    #[inline]
    pub fn set_editable(&mut self, editable: bool) {
        self.inner.set_editable(editable);
    }

    /// Set whether this `EditBox` uses multi-line mode
    ///
    /// This setting has two effects: the vertical size allocation is increased
    /// and wrapping is enabled if true. Default: false.
    ///
    /// This method is ineffective if the text class is set by
    /// [`Self::with_class`] to anything other than [`TextClass::Edit`].
    #[inline]
    #[must_use]
    pub fn with_multi_line(mut self, multi_line: bool) -> Self {
        self.inner = self.inner.with_multi_line(multi_line);
        self
    }

    /// True if the editor uses multi-line mode
    ///
    /// See also: [`Self::with_multi_line`]
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.inner.multi_line()
    }

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.inner = self.inner.with_class(class);
        self
    }

    /// Get the text class used
    #[inline]
    pub fn class(&self) -> TextClass {
        self.inner.class()
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

    /// Get whether the widget has edit focus
    ///
    /// This is true when the widget is editable and has keyboard focus.
    #[inline]
    pub fn has_edit_focus(&self) -> bool {
        self.inner.has_edit_focus()
    }

    /// Get whether the input state is erroneous
    #[inline]
    pub fn has_error(&self) -> bool {
        self.inner.has_error()
    }

    /// Set the error state
    ///
    /// When true, the input field's background is drawn red.
    /// This state is cleared by [`Self::set_string`].
    pub fn set_error_state(&mut self, cx: &mut EventState, error_state: bool) {
        self.inner.set_error_state(cx, error_state);
    }
}
