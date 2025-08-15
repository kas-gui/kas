// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use crate::{ScrollBar, ScrollMsg};
use kas::event::components::{ScrollComponent, TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, ElementState, FocusSource, ImePurpose, PhysicalKey, Scroll};
use kas::geom::Vec2;
use kas::messages::{ReplaceSelectedText, SetValueText};
use kas::prelude::*;
use kas::text::{NotReady, SelectionHelper};
use kas::theme::{Background, FrameStyle, Text, TextClass};
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::Range;
use std::str::FromStr;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

#[derive(Clone, Debug, Default, PartialEq)]
enum LastEdit {
    #[default]
    None,
    Insert,
    Delete,
    Paste,
}

enum EditAction {
    None,
    Activate,
    Edit,
}

/// Event-handling *guard* for [`EditField`], [`EditBox`]
///
/// This is the most generic interface; see also constructors of [`EditField`],
/// [`EditBox`] for common use-cases.
///
/// All methods on this trait are passed a reference to the [`EditField`] as
/// parameter. The guard itself is a public field: `edit.guard`.
///
/// All methods have a default implementation which does nothing.
pub trait EditGuard: Sized {
    /// Data type
    type Data;

    /// Configure guard
    ///
    /// This function is called when the attached widget is configured.
    fn configure(edit: &mut EditField<Self>, cx: &mut ConfigCx) {
        let _ = (edit, cx);
    }

    /// Update guard
    ///
    /// This function is called when input data is updated.
    ///
    /// Note that this method may be called during editing as a result of a
    /// message sent by [`Self::edit`] or another cause. It is recommended to
    /// ignore updates for editable widgets with key focus
    /// ([`EditField::has_edit_focus`]) to avoid overwriting user input;
    /// [`Self::focus_lost`] may update the content instead.
    /// For read-only fields this is not recommended (but `has_edit_focus` will
    /// not be true anyway).
    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation:
    ///
    /// -   If the field is editable, calls [`Self::focus_lost`] and returns
    ///     returns [`Used`].
    /// -   If the field is not editable, returns [`Unused`].
    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Self::Data) -> IsUsed {
        if edit.editable {
            Self::focus_lost(edit, cx, data);
            Used
        } else {
            Unused
        }
    }

    /// Focus-gained guard
    ///
    /// This function is called when the widget gains keyboard input focus.
    fn focus_gained(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Focus-lost guard
    ///
    /// This function is called when the widget loses keyboard input focus.
    fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Edit guard
    ///
    /// This function is called when contents are updated by the user.
    fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }
}

/// Ignore all events and data updates
///
/// This guard should probably not be used for a functional user-interface but
/// may be useful in mock UIs.
#[autoimpl(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DefaultGuard<A>(PhantomData<A>);
impl<A: 'static> EditGuard for DefaultGuard<A> {
    type Data = A;
}

#[impl_self]
mod StringGuard {
    /// An [`EditGuard`] for read-only strings
    ///
    /// This may be used with read-only edit fields, essentially resulting in a
    /// fancier version of [`Text`](crate::Text) or
    /// [`ScrollText`](crate::ScrollText).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct StringGuard<A> {
        value_fn: Box<dyn Fn(&A) -> String>,
        on_afl: Option<Box<dyn Fn(&mut EventCx, &A, &str)>>,
        edited: bool,
    }

    impl Self {
        /// Construct with a value function
        ///
        /// On update, `value_fn` is used to extract a value from input data.
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// No other action happens unless [`Self::with_msg`] is used.
        pub fn new(value_fn: impl Fn(&A) -> String + 'static) -> Self {
            StringGuard {
                value_fn: Box::new(value_fn),
                on_afl: None,
                edited: false,
            }
        }

        /// Call the handler `f` on activation / focus loss
        ///
        /// On field **a**ctivation and **f**ocus **l**oss (AFL) after an edit,
        /// `f` is called.
        pub fn with(mut self, f: impl Fn(&mut EventCx, &A, &str) + 'static) -> Self {
            debug_assert!(self.on_afl.is_none());
            self.on_afl = Some(Box::new(f));
            self
        }

        /// Send the message generated by `f` on activation / focus loss
        ///
        /// On field **a**ctivation and **f**ocus **l**oss (AFL) after an edit,
        /// `f` is used to construct a message to be emitted via [`EventCx::push`].
        pub fn with_msg<M: Debug + 'static>(self, f: impl Fn(&str) -> M + 'static) -> Self {
            self.with(move |cx, _, value| cx.push(f(value)))
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
            if edit.guard.edited {
                edit.guard.edited = false;
                if let Some(ref on_afl) = edit.guard.on_afl {
                    return on_afl(cx, data, edit.as_str());
                }
            }

            // Reset data on focus loss (update is inhibited with focus).
            // No need if we just sent a message (should cause an update).
            let string = (edit.guard.value_fn)(data);
            edit.set_string(cx, string);
        }

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let string = (edit.guard.value_fn)(data);
                edit.set_string(cx, string);
            }
        }

        fn edit(edit: &mut EditField<Self>, _: &mut EventCx, _: &Self::Data) {
            edit.guard.edited = true;
        }
    }
}

#[impl_self]
mod ParseGuard {
    /// An [`EditGuard`] for parsable types
    ///
    /// This guard displays a value formatted from input data, updates the error
    /// state according to parse success on each keystroke, and sends a message
    /// on focus loss (where successful parsing occurred).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct ParseGuard<A, T: Debug + Display + FromStr> {
        parsed: Option<T>,
        value_fn: Box<dyn Fn(&A) -> T>,
        on_afl: Box<dyn Fn(&mut EventCx, T)>,
    }

    impl Self {
        /// Construct
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
        pub fn new<M: Debug + 'static>(
            value_fn: impl Fn(&A) -> T + 'static,
            on_afl: impl Fn(T) -> M + 'static,
        ) -> Self {
            ParseGuard {
                parsed: None,
                value_fn: Box::new(value_fn),
                on_afl: Box::new(move |cx, value| cx.push(on_afl(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
            if let Some(value) = edit.guard.parsed.take() {
                (edit.guard.on_afl)(cx, value);
            } else {
                // Reset data on focus loss (update is inhibited with focus).
                // No need if we just sent a message (should cause an update).
                let value = (edit.guard.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
            }
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
            edit.guard.parsed = edit.as_str().parse().ok();
            edit.set_error_state(cx, edit.guard.parsed.is_none());
        }

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let value = (edit.guard.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
                edit.guard.parsed = None;
            }
        }
    }
}

#[impl_self]
mod InstantParseGuard {
    /// An as-you-type [`EditGuard`] for parsable types
    ///
    /// This guard displays a value formatted from input data, updates the error
    /// state according to parse success on each keystroke, and sends a message
    /// immediately (where successful parsing occurred).
    #[autoimpl(Debug ignore self.value_fn, self.on_afl)]
    pub struct InstantParseGuard<A, T: Debug + Display + FromStr> {
        value_fn: Box<dyn Fn(&A) -> T>,
        on_afl: Box<dyn Fn(&mut EventCx, T)>,
    }

    impl Self {
        /// Construct
        ///
        /// On update, `value_fn` is used to extract a value from input data
        /// which is then formatted as a string via [`Display`].
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// On every edit, the guard attempts to parse the field's input as type
        /// `T` via [`FromStr`]. On success, the result is converted to a
        /// message via `on_afl` then emitted via [`EventCx::push`].
        pub fn new<M: Debug + 'static>(
            value_fn: impl Fn(&A) -> T + 'static,
            on_afl: impl Fn(T) -> M + 'static,
        ) -> Self {
            InstantParseGuard {
                value_fn: Box::new(value_fn),
                on_afl: Box::new(move |cx, value| cx.push(on_afl(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
            // Always reset data on focus loss
            let value = (edit.guard.value_fn)(data);
            edit.set_string(cx, format!("{value}"));
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
            let result = edit.as_str().parse();
            edit.set_error_state(cx, result.is_err());
            if let Ok(value) = result {
                (edit.guard.on_afl)(cx, value);
            }
        }

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
            if !edit.has_edit_focus() {
                let value = (edit.guard.value_fn)(data);
                edit.set_string(cx, format!("{value}"));
            }
        }
    }
}

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
        inner_margin: i32,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.sub_other(self.frame_size.extract(axis.flipped()));

            let mut rules = self.inner.size_rules(sizer.re(), axis);
            let bar_rules = self.vert_bar.size_rules(sizer.re(), axis);
            if axis.is_horizontal() && self.multi_line() {
                self.inner_margin = rules.margins_i32().1.max(bar_rules.margins_i32().0);
                rules.append(bar_rules);
            }

            let frame_rules = sizer.frame(FrameStyle::EditBox, axis);
            let (rules, offset, size) = frame_rules.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, outer_rect: Rect, hints: AlignHints) {
            widget_set_rect!(outer_rect);
            let mut rect = outer_rect;
            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;

            let mut bar_rect = Rect::ZERO;
            if self.multi_line() {
                let bar_width = cx.size_cx().scroll_bar_width();
                let x1 = rect.pos.0 + rect.size.0;
                let x0 = x1 - bar_width;
                bar_rect = Rect::new(Coord(x0, rect.pos.1), Size(bar_width, rect.size.1));
                rect.size.0 = (rect.size.0 - bar_width - self.inner_margin).max(0);
            }
            self.vert_bar.set_rect(cx, bar_rect, AlignHints::NONE);

            self.inner.set_rect(cx, rect, hints);
            let _ = self
                .scroll
                .set_sizes(self.rect().size, self.inner.rect().size);
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

            draw_inner.with_clip_region(self.rect(), self.scroll.offset(), |draw| {
                self.inner.draw(draw);
            });
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
    }

    impl Events for Self {
        type Data = G::Data;

        fn handle_event(&mut self, cx: &mut EventCx, _: &Self::Data, event: Event) -> IsUsed {
            let used = self
                .scroll
                .scroll_by_event(cx, event, self.id(), self.rect());
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
            // TODO: pass ReplaceSelectedText to inner widget?
        }

        fn handle_scroll(&mut self, cx: &mut EventCx<'_>, _: &G::Data, scroll: Scroll) {
            // Inner may have resized itself, hence we update sizes now.
            let _ = self
                .scroll
                .set_sizes(self.rect().size, self.inner.rect().size);
            self.scroll.scroll(cx, self.id(), self.rect(), scroll);
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
                inner_margin: Default::default(),
            }
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

        /// Set text contents from a `String`
        ///
        /// This method does not call action handlers on the [`EditGuard`].
        #[inline]
        pub fn set_string(&mut self, cx: &mut EventState, text: String) {
            self.inner.set_string(cx, text);
        }

        /// Access the edit guard
        #[inline]
        pub fn guard(&self) -> &G {
            &self.inner.guard
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
        self.inner.editable = true;
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

/// Used to track ongoing incompatible actions
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CurrentAction {
    #[default]
    None,
    DragSelect,
    ImeStart,
    ImeEdit,
}

impl CurrentAction {
    fn is_select(self) -> bool {
        matches!(self, CurrentAction::DragSelect)
    }

    fn is_ime(self) -> bool {
        matches!(self, CurrentAction::ImeStart | CurrentAction::ImeEdit)
    }

    fn is_active_ime(self) -> bool {
        false
    }

    fn clear_active(&mut self) {
        if matches!(self, CurrentAction::DragSelect | CurrentAction::ImeEdit) {
            *self = CurrentAction::None;
        }
    }

    fn clear_selection(&mut self) {
        if matches!(self, CurrentAction::DragSelect) {
            *self = CurrentAction::None;
        }
    }
}

#[impl_self]
mod EditField {
    /// A text-edit field (single- or multi-line)
    ///
    /// This widget implements the mechanics of text layout and event handling.
    /// If you want a box with a border, use [`EditBox`] instead.
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
            if self.current.is_ime() {
                self.set_ime_cursor_area(cx);
            }
        }

        fn draw(&self, mut draw: DrawCx) {
            let rect = self.rect();

            // TODO(opt): we could cache the selection rectangles here to make
            // drawing more efficient (self.text.highlight_lines(range) output).
            // The same applies to the edit marker below.
            draw.text_selected(rect, &self.text, self.selection.range());

            if self.editable && draw.ev_state().has_key_focus(self.id_ref()).0 {
                draw.text_cursor(rect, &self.text, self.selection.edit_index());
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
                cursor: self.selection.edit_index(),
                sel_index: self.selection.sel_index(),
            }
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Events for Self {
        const REDRAW_ON_MOUSE_OVER: bool = true;

        type Data = G::Data;

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
                    TextInputAction::Focus { coord, action }
                        if self.current.is_select() || action.anchor =>
                    {
                        if self.current.is_ime() {
                            cx.cancel_ime_focus(self.id());
                        }
                        self.current = CurrentAction::DragSelect;
                        self.set_cursor_from_coord(cx, coord);
                        self.selection.action(&self.text, action);

                        if self.has_key_focus {
                            self.set_primary(cx);
                        }
                        Used
                    }
                    TextInputAction::Finish if self.current.is_select() => {
                        self.current = CurrentAction::None;
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

        /// Set text contents from a `String`
        ///
        /// This method does not call action handlers on the [`EditGuard`].
        ///
        /// NOTE: we could add a `set_str` variant of this method but there
        /// doesn't appear to be a need.
        pub fn set_string(&mut self, cx: &mut EventState, string: String) {
            if !self.text.set_string(string) || !self.text.prepare() {
                return;
            }

            self.current.clear_active();
            self.selection.set_max_len(self.text.str_len());
            cx.redraw(&self);
            if self.current.is_ime() {
                self.set_ime_cursor_area(cx);
            }
            self.set_error_state(cx, false);
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
