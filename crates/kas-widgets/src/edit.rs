// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! The [`EditField`] and [`EditBox`] widgets, plus supporting items

use crate::{ScrollBar, ScrollMsg};
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{Command, CursorIcon, Scroll, ScrollDelta};
use kas::geom::Vec2;
use kas::prelude::*;
use kas::text::{NotReady, SelectionHelper, Text};
use kas::theme::{Background, FrameStyle, TextClass};
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
    Unused,
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
    /// message sent by [`Self::edit`] or another cause, thus usually this
    /// method should do nothing if [`EditField::has_key_focus`]. Instead, it
    /// may be desirable to update content on [`Self::focus_lost`].
    fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &Self::Data) {
        let _ = (edit, cx, data);
    }

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation returns [`Response::Unused`].
    fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, data: &Self::Data) -> Response {
        let _ = (edit, cx, data);
        Response::Unused
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

impl_scope! {
    /// An [`EditGuard`] impl for string input
    #[autoimpl(Debug ignore self.value_fn, self.msg_fn)]
    pub struct StringGuard<A> {
        value_fn: Box<dyn Fn(&A) -> String>,
        msg_fn: Option<Box<dyn Fn(&mut EventCx, &str)>>,
    }

    impl Self {
        /// Construct with a value function
        ///
        /// On update, `value_fn` is used to extract a value from input data.
        /// If, however, the input field has focus, the update is ignored.
        ///
        /// No other action happens unless [`Self::on_afl`] is used.
        pub fn new(value_fn: impl Fn(&A) -> String + 'static) -> Self {
            StringGuard {
                value_fn: Box::new(value_fn),
                msg_fn: None,
            }
        }

        /// Set a message function
        ///
        /// On field **a**ctivation and **f**ocus **l**oss (AFL), `msg_fn` is
        /// used to construct a message to be emitted via [`EventCx::push`].
        ///
        /// There is no message de-duplication: a message is sent each time the
        /// field is activated or loses focus even if content remains unchanged.
        /// TODO: should we change this behaviour to match [`ParseGuard`] which
        /// does de-duplicate messages?
        pub fn on_afl<M: Debug + 'static>(mut self, msg_fn: impl Fn(&str) -> M + 'static) -> Self {
            debug_assert!(self.msg_fn.is_none());
            self.msg_fn = Some(Box::new(move |cx, value| cx.push(msg_fn(value))));
            self
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) -> Response {
            Self::focus_lost(edit, cx, data);
            Response::Used
        }

        fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
            if let Some(ref msg_fn) = edit.guard.msg_fn {
                msg_fn(cx, edit.get_str());
            } else {
                // Reset data on focus loss (update is inhibited with focus).
                // We do not do this given a msg_fn since that is expected
                // to adjust data, thus triggering update with new data.
                let string = (edit.guard.value_fn)(data);
                *cx |= edit.set_string(string);
            }
        }

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
            if !edit.has_key_focus() {
                let string = (edit.guard.value_fn)(data);
                *cx |= edit.set_string(string);
            }
        }
    }
}

impl_scope! {
    /// An [`EditGuard`] impl for simple parsable types (e.g. numbers)
    #[autoimpl(Debug ignore self.value_fn, self.msg_fn)]
    pub struct ParseGuard<A, T: Debug + Display + FromStr> {
        parsed: Option<T>,
        value_fn: Box<dyn Fn(&A) -> T>,
        msg_fn: Box<dyn Fn(&mut EventCx, T)>,
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
        /// previous paragraph), `msg_fn` is used to construct a message to be
        /// emitted via [`EventCx::push`].
        ///
        /// The cached value is cleared when a message is sent by activation or
        /// focus loss to avoid duplicate messages. TODO: should we change this
        /// behaviour to match [`StringGuard`] which does not de-duplicate?
        pub fn new<M: Debug + 'static>(
            value_fn: impl Fn(&A) -> T + 'static,
            msg_fn: impl Fn(T) -> M + 'static,
        ) -> Self {
            ParseGuard {
                parsed: None,
                value_fn: Box::new(value_fn),
                msg_fn: Box::new(move |cx, value| cx.push(msg_fn(value))),
            }
        }
    }

    impl EditGuard for Self {
        type Data = A;

        fn activate(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) -> Response {
            Self::focus_lost(edit, cx, data);
            Response::Used
        }

        fn focus_lost(edit: &mut EditField<Self>, cx: &mut EventCx, data: &A) {
            if let Some(value) = edit.guard.parsed.take() {
                (edit.guard.msg_fn)(cx, value);
            } else {
                // Reset data on focus loss (update is inhibited with focus).
                // We do not do this given parsed value since msg_fn is expected
                // to adjust data, thus triggering update with new data.
                let value = (edit.guard.value_fn)(data);
                *cx |= edit.set_string(format!("{}", value));
            }
        }

        fn edit(edit: &mut EditField<Self>, cx: &mut EventCx, _: &A) {
            edit.guard.parsed = edit.get_str().parse().ok();
            *cx |= edit.set_error_state(edit.guard.parsed.is_none());
        }

        fn update(edit: &mut EditField<Self>, cx: &mut ConfigCx, data: &A) {
            if !edit.has_key_focus() {
                let value = (edit.guard.value_fn)(data);
                *cx |= edit.set_string(format!("{}", value));
                edit.guard.parsed = None;
            }
        }
    }
}

impl_scope! {
    /// A text-edit box
    ///
    /// A single- or multi-line editor for unformatted text.
    /// See also notes on [`EditField`].
    ///
    /// By default, the editor supports a single-line only;
    /// [`Self::with_multi_line`] and [`Self::with_class`] can be used to change this.
    #[autoimpl(Deref, DerefMut, HasStr, HasString using self.inner)]
    #[autoimpl(Clone, Default, Debug where G: trait)]
    #[widget]
    pub struct EditBox<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        #[widget]
        inner: EditField<G>,
        #[widget(&())]
        bar: ScrollBar<kas::dir::Down>,
        frame_offset: Offset,
        frame_size: Size,
        inner_margin: i32,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, mut axis: AxisInfo) -> SizeRules {
            axis.sub_other(self.frame_size.extract(axis.flipped()));

            let mut rules = self.inner.size_rules(sizer.re(), axis);
            if axis.is_horizontal() && self.multi_line() {
                let bar_rules = self.bar.size_rules(sizer.re(), axis);
                self.inner_margin = rules.margins_i32().1.max(bar_rules.margins_i32().0);
                rules.append(bar_rules);
            }

            let frame_rules = sizer.frame(FrameStyle::EditBox, axis);
            let (rules, offset, size) = frame_rules.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, mut rect: Rect) {
            self.core.rect = rect;
            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;
            if self.multi_line() {
                let bar_width = cx.size_cx().scroll_bar_width();
                let x1 = rect.pos.0 + rect.size.0;
                let x0 = x1 - bar_width;
                let bar_rect = Rect::new(Coord(x0, rect.pos.1), Size(bar_width, rect.size.1));
                self.bar.set_rect(cx, bar_rect);
                rect.size.0 = (rect.size.0 - bar_width - self.inner_margin).max(0);
            }
            self.inner.set_rect(cx, rect);
            self.update_scroll_bar(cx);
        }

        fn find_id(&mut self, coord: Coord) -> Option<WidgetId> {
            if !self.rect().contains(coord) {
                return None;
            }

            if self.max_scroll_offset().1 > 0 {
                if let Some(id) = self.bar.find_id(coord) {
                    return Some(id);
                }
            }

            // If coord is over self but not over self.bar, we assign
            // the event to self.inner without further question.
            Some(self.inner.id())
        }

        fn draw(&mut self, mut draw: DrawCx) {
            if self.max_scroll_offset().1 > 0 {
                draw.recurse(&mut self.bar);
            }
            let mut draw = draw.re_id(self.inner.id());
            let bg = if self.inner.has_error() {
                Background::Error
            } else {
                Background::Default
            };
            draw.frame(self.rect(), FrameStyle::EditBox, bg);
            self.inner.draw(draw);
        }
    }

    impl Events for Self {
        type Data = G::Data;

        fn handle_messages(&mut self, cx: &mut EventCx<'_>, _: &G::Data) {
            if let Some(ScrollMsg(y)) = cx.try_pop() {
                self.inner
                    .set_scroll_offset(cx, Offset(self.inner.view_offset.0, y));
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx<'_>, _: &G::Data, _: Scroll) {
            self.update_scroll_bar(cx);
        }
    }

    impl Scrollable for Self {
        #[inline]
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            self.inner.scroll_axes(size)
        }

        #[inline]
        fn max_scroll_offset(&self) -> Offset {
            self.inner.max_scroll_offset()
        }

        #[inline]
        fn scroll_offset(&self) -> Offset {
            self.inner.scroll_offset()
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let offset = self.inner.set_scroll_offset(cx, offset);
            self.update_scroll_bar(cx);
            offset
        }
    }

    impl Self {
        fn update_scroll_bar(&mut self, cx: &mut EventState) {
            let max_offset = self.inner.max_scroll_offset().1;
            *cx |= self.bar.set_limits(max_offset, self.inner.rect().size.1);
            self.bar.set_value(cx, self.inner.view_offset.1);
        }
    }

    impl ToString for Self {
        fn to_string(&self) -> String {
            self.inner.to_string()
        }
    }
}

impl<G: EditGuard> EditBox<G> {
    /// Construct an `EditBox` with an [`EditGuard`]
    #[inline]
    pub fn new(guard: G) -> EditBox<G> {
        EditBox {
            core: Default::default(),
            inner: EditField::new(guard),
            bar: Default::default(),
            frame_offset: Default::default(),
            frame_size: Default::default(),
            inner_margin: Default::default(),
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

    /// Construct a read-only `EditBox` for text content
    #[inline]
    pub fn ro(value_fn: impl Fn(&A) -> String + 'static) -> EditBox<StringGuard<A>> {
        EditBox::new(StringGuard::new(value_fn)).with_editable(false)
    }

    /// Construct a read-write `EditBox` for text content
    #[inline]
    pub fn rw<M: Debug + 'static>(
        value_fn: impl Fn(&A) -> String + 'static,
        msg_fn: impl Fn(&str) -> M + 'static,
    ) -> EditBox<StringGuard<A>> {
        EditBox::new(StringGuard::new(value_fn).on_afl(msg_fn))
    }

    /// Construct an `EditBox` for a parsable value (e.g. a number)
    #[inline]
    pub fn parser<T: Debug + Display + FromStr, M: Debug + 'static>(
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
    ) -> EditBox<ParseGuard<A, T>> {
        EditBox::new(ParseGuard::new(value_fn, msg_fn))
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

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.inner = self.inner.with_class(class);
        self
    }

    /// Adjust the height allocation (inline)
    #[inline]
    #[must_use]
    pub fn with_lines(mut self, min_lines: i32, ideal_lines: i32) -> Self {
        self.set_lines(min_lines, ideal_lines);
        self
    }

    /// Adjust the width allocation (inline)
    #[inline]
    #[must_use]
    pub fn with_width_em(mut self, min_em: f32, ideal_em: f32) -> Self {
        self.set_width_em(min_em, ideal_em);
        self
    }
}

impl_scope! {
    /// A text-edit field (single- or multi-line)
    ///
    /// This widget implements the mechanics of text layout and event handling.
    /// It does not draw any background (even to indicate an error state) or
    /// borders (even to indicate focus), thus usually it is better to use a
    /// derived type like [`EditBox`] instead.
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
    #[impl_default(where G: Default)]
    #[autoimpl(Clone, Debug where G: trait)]
    #[widget{
        navigable = true;
        hover_highlight = true;
        cursor_icon = CursorIcon::Text;
    }]
    pub struct EditField<G: EditGuard = DefaultGuard<()>> {
        core: widget_core!(),
        view_offset: Offset,
        editable: bool,
        class: TextClass = TextClass::Edit(false),
        align: AlignPair,
        width: (f32, f32) = (8.0, 16.0),
        lines: (i32, i32) = (1, 1),
        text: Text<String>,
        text_size: Size,
        selection: SelectionHelper,
        edit_x_coord: Option<f32>,
        old_state: Option<(String, usize, usize)>,
        last_edit: LastEdit,
        has_key_focus: bool,
        error_state: bool,
        input_handler: TextInput,
        /// The associated [`EditGuard`] implementation
        pub guard: G,
    }

    impl Layout for Self {
        fn size_rules(&mut self, sizer: SizeCx, axis: AxisInfo) -> SizeRules {
            let (min, ideal) = if axis.is_horizontal() {
                let dpem = sizer.dpem();
                ((self.width.0 * dpem).cast_ceil(), (self.width.1 * dpem).cast_ceil())
            } else {
                let height = sizer.line_height(self.class);
                (self.lines.0 * height, self.lines.1 * height)
            };
            let margins = sizer.text_margins().extract(axis);
            let (stretch, align) = if axis.is_horizontal() || self.multi_line() {
                (Stretch::High, axis.align_or_default())
            } else {
                (Stretch::None, axis.align_or_center())
            };
            self.align.set_component(axis, align);
            SizeRules::new(min, ideal, margins, stretch)
        }

        fn set_rect(&mut self, cx: &mut ConfigCx, rect: Rect) {
            self.core.rect = rect;
            cx.text_set_size(&mut self.text, self.class, rect.size, Some(self.align));
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();
            self.view_offset = self.view_offset.min(self.max_scroll_offset());
        }

        fn draw(&mut self, mut draw: DrawCx) {
            let mut rect = self.rect();
            rect.size = rect.size.max(self.text_size);
            draw.with_clip_region(self.rect(), self.view_offset, |mut draw| {
                if self.selection.is_empty() {
                    draw.text(rect, &self.text, self.class);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(
                        rect,
                        &self.text,
                        self.selection.range(),
                        self.class,
                    );
                }
                if self.editable && draw.ev_state().has_char_focus(self.id_ref()).0 {
                    draw.text_cursor(
                        rect,
                        &self.text,
                        self.class,
                        self.selection.edit_pos(),
                    );
                }
            });
        }
    }

    impl Events for Self {
        type Data = G::Data;

        fn configure(&mut self, cx: &mut ConfigCx) {
            G::configure(self, cx);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &G::Data) {
            G::update(self, cx, data);
        }

        fn handle_event(&mut self, cx: &mut EventCx, data: &G::Data, event: Event) -> Response {
            fn request_focus<G: EditGuard>(s: &mut EditField<G>, cx: &mut EventCx, data: &G::Data) {
                if !s.has_key_focus && cx.request_char_focus(s.id()) {
                    s.has_key_focus = true;
                    cx.set_scroll(Scroll::Rect(s.rect()));
                    G::focus_gained(s, cx, data);
                }
            }

            match event {
                Event::NavFocus(true) => {
                    request_focus(self, cx, data);
                    if !self.class.multi_line() {
                        self.selection.clear();
                        self.selection.set_edit_pos(self.text.str_len());
                        cx.redraw(self.id());
                    }
                    Response::Used
                }
                Event::NavFocus(false) => Response::Used,
                Event::LostNavFocus => {
                    if !self.class.multi_line() {
                        self.selection.set_empty();
                        cx.redraw(self.id());
                    }
                    Response::Used
                }
                Event::LostCharFocus => {
                    self.has_key_focus = false;
                    cx.redraw(self.id());
                    G::focus_lost(self, cx, data);
                    Response::Used
                }
                Event::LostSelFocus => {
                    self.selection.set_empty();
                    cx.redraw(self.id());
                    Response::Used
                }
                Event::Command(cmd) => {
                    // Note: we can receive a Command without char focus, but should
                    // ensure we have focus before acting on it.
                    request_focus(self, cx, data);
                    if self.has_key_focus {
                        match self.control_key(cx, cmd) {
                            Ok(EditAction::None) => Response::Used,
                            Ok(EditAction::Unused) => Response::Unused,
                            Ok(EditAction::Activate) => G::activate(self, cx, data),
                            Ok(EditAction::Edit) => {
                                G::edit(self, cx, data);
                                Response::Used
                            }
                            Err(NotReady) => Response::Used,
                        }
                    } else {
                        Response::Unused
                    }
                }
                Event::ReceivedCharacter(c) => match self.received_char(cx, c) {
                    false => Response::Unused,
                    true => {
                        G::edit(self, cx, data);
                        Response::Used
                    }
                },
                Event::Scroll(delta) => {
                    let delta2 = match delta {
                        ScrollDelta::LineDelta(x, y) => cx.config().scroll_distance((x, y)),
                        ScrollDelta::PixelDelta(coord) => coord,
                    };
                    self.pan_delta(cx, delta2)
                }
                Event::PressStart { press } if press.is_tertiary() =>
                    press.grab(self.id())
                        .with_mode(kas::event::GrabMode::Click)
                        .with_cx(cx),
                Event::PressEnd { press, .. } if press.is_tertiary() => {
                    if let Some(content) = cx.get_primary() {
                        self.set_edit_pos_from_coord(cx, press.coord);
                        self.selection.set_empty();
                        let pos = self.selection.edit_pos();
                        let range = self.trim_paste(&content);
                        let len = range.len();

                        self.old_state =
                            Some((self.text.clone_string(), pos, self.selection.sel_pos()));
                        self.last_edit = LastEdit::Paste;

                        self.text.replace_range(pos..pos, &content[range]);
                        self.selection.set_pos(pos + len);
                        self.edit_x_coord = None;
                        self.prepare_text(cx);

                        G::edit(self, cx, data);
                    }
                    Response::Used
                }
                event => match self.input_handler.handle(cx, self.id(), event) {
                    TextInputAction::None => Response::Used,
                    TextInputAction::Unused => Response::Unused,
                    TextInputAction::Pan(delta) => self.pan_delta(cx, delta),
                    TextInputAction::Focus => {
                        request_focus(self, cx, data);
                        Response::Used
                    }
                    TextInputAction::Cursor(coord, anchor, clear, repeats) => {
                        request_focus(self, cx, data);
                        if self.has_key_focus {
                            self.set_edit_pos_from_coord(cx, coord);
                            if anchor {
                                self.selection.set_anchor();
                            }
                            if clear {
                                self.selection.set_empty();
                            }
                            if repeats > 1 {
                                self.selection.expand(&self.text, repeats);
                            }
                            self.set_primary(cx);
                        }
                        Response::Used
                    }
                },
            }
        }
    }

    impl Scrollable for Self {
        fn scroll_axes(&self, size: Size) -> (bool, bool) {
            let max = self.max_scroll_offset();
            (max.0 > size.0, max.1 > size.1)
        }

        fn max_scroll_offset(&self) -> Offset {
            let text_size = Offset::conv(self.text_size);
            let self_size = Offset::conv(self.rect().size);
            (text_size - self_size).max(Offset::ZERO)
        }

        fn scroll_offset(&self) -> Offset {
            self.view_offset
        }

        fn set_scroll_offset(&mut self, cx: &mut EventCx, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.view_offset = new_offset;
                // No widget moves so do not need to report Action::REGION_MOVED
                cx.redraw(self.id());
            }
            new_offset
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.text.text()
        }
    }

    impl HasString for Self {
        fn set_string(&mut self, string: String) -> Action {
            if *self.text.text() == string {
                return Action::empty();
            }
            let mut action = Action::REDRAW;

            let len = string.len();
            self.text.set_string(string);
            self.selection.set_max_len(len);
            if self.text.try_prepare().is_ok() {
                self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();
                self.view_offset = self.view_offset.min(self.max_scroll_offset());
                // We use SET_RECT just to set the outer scroll bar position:
                action = Action::SET_RECT;
            }
            action | self.set_error_state(false)
        }
    }

    impl ToString for Self {
        fn to_string(&self) -> String {
            self.text.text().clone()
        }
    }
}

impl<G: EditGuard> EditField<G> {
    /// Construct an `EditBox` with an [`EditGuard`]
    #[inline]
    pub fn new(guard: G) -> EditField<G> {
        EditField {
            core: Default::default(),
            view_offset: Default::default(),
            editable: true,
            class: TextClass::Edit(false),
            align: Default::default(),
            width: (8.0, 16.0),
            lines: (1, 1),
            text: Default::default(),
            text_size: Default::default(),
            selection: Default::default(),
            edit_x_coord: None,
            old_state: None,
            last_edit: Default::default(),
            has_key_focus: false,
            error_state: false,
            input_handler: Default::default(),
            guard,
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
            class: TextClass::Edit(false),
            text: Text::new(text),
            selection: SelectionHelper::new(len, len),
            ..Default::default()
        }
    }

    /// Construct a read-only `EditField` for text content
    #[inline]
    pub fn ro(value_fn: impl Fn(&A) -> String + 'static) -> EditField<StringGuard<A>> {
        EditField::new(StringGuard::new(value_fn)).with_editable(false)
    }

    /// Construct a read-write `EditField` for text content
    #[inline]
    pub fn rw<M: Debug + 'static>(
        value_fn: impl Fn(&A) -> String + 'static,
        msg_fn: impl Fn(&str) -> M + 'static,
    ) -> EditField<StringGuard<A>> {
        EditField::new(StringGuard::new(value_fn).on_afl(msg_fn))
    }

    /// Construct an `EditField` for a parsable value (e.g. a number)
    #[inline]
    pub fn parser<T: Debug + Display + FromStr, M: Debug + 'static>(
        value_fn: impl Fn(&A) -> T + 'static,
        msg_fn: impl Fn(T) -> M + 'static,
    ) -> EditField<ParseGuard<A, T>> {
        EditField::new(ParseGuard::new(value_fn, msg_fn))
    }
}

impl<G: EditGuard> EditField<G> {
    /// Set the initial text (inline)
    ///
    /// This method should only be used on a new `EditBox`.
    #[inline]
    #[must_use]
    pub fn with_text(mut self, text: impl ToString) -> Self {
        let text = text.to_string();
        let len = text.len();
        self.text.set_string(text);
        self.selection.set_pos(len);
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
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether this `EditField` is editable
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
        self.class = TextClass::Edit(multi_line);
        self.lines = match multi_line {
            false => (1, 1),
            true => (4, 7),
        };
        self
    }

    /// True if the editor uses multi-line mode
    ///
    /// See also: [`Self::with_multi_line`]
    #[inline]
    pub fn multi_line(&self) -> bool {
        self.class.multi_line()
    }

    /// Set the text class used
    #[inline]
    #[must_use]
    pub fn with_class(mut self, class: TextClass) -> Self {
        self.class = class;
        self
    }

    /// Get the text class used
    #[inline]
    pub fn class(&self) -> TextClass {
        self.class
    }

    /// Adjust the height allocation
    #[inline]
    pub fn set_lines(&mut self, min_lines: i32, ideal_lines: i32) {
        self.lines = (min_lines, ideal_lines);
    }

    /// Adjust the height allocation (inline)
    #[inline]
    #[must_use]
    pub fn with_lines(mut self, min_lines: i32, ideal_lines: i32) -> Self {
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

    /// Get whether the widget currently has keyboard input focus
    #[inline]
    pub fn has_key_focus(&self) -> bool {
        self.has_key_focus
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
    pub fn set_error_state(&mut self, error_state: bool) -> Action {
        self.error_state = error_state;
        Action::REDRAW
    }

    fn prepare_text(&mut self, cx: &mut EventCx) {
        if !self.text.required_action().is_ready() {
            let start = std::time::Instant::now();

            self.text.prepare().expect("invalid font_id");
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();

            log::trace!(
                target: "kas_perf::widgets::edit", "prepare_text: {}μs",
                start.elapsed().as_micros(),
            );
        }

        cx.redraw(self.id());
        self.set_view_offset_from_edit_pos(cx);
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

    // returns true on success, false on unhandled event
    fn received_char(&mut self, cx: &mut EventCx, c: char) -> bool {
        if !self.editable {
            return false;
        }

        let pos = self.selection.edit_pos();
        let selection = self.selection.range();
        let have_sel = selection.start < selection.end;
        if self.last_edit != LastEdit::Insert || have_sel {
            self.old_state = Some((self.text.clone_string(), pos, self.selection.sel_pos()));
            self.last_edit = LastEdit::Insert;
        }
        if have_sel {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            self.text.replace_range(selection.clone(), s);
            self.selection.set_pos(selection.start + s.len());
        } else {
            self.text.insert_char(pos, c);
            self.selection.set_pos(pos + c.len_utf8());
        }
        self.edit_x_coord = None;

        self.prepare_text(cx);
        true
    }

    fn control_key(&mut self, cx: &mut EventCx, key: Command) -> Result<EditAction, NotReady> {
        let editable = self.editable;
        let mut shift = cx.modifiers().shift();
        let mut buf = [0u8; 4];
        let pos = self.selection.edit_pos();
        let len = self.text.str_len();
        let multi_line = self.multi_line();
        let selection = self.selection.range();
        let have_sel = selection.end > selection.start;
        let string;

        enum Action<'a> {
            None,
            Unused,
            Activate,
            Edit,
            Insert(&'a str, LastEdit),
            Delete(Range<usize>),
            Move(usize, Option<f32>),
        }

        let action = match key {
            Command::Escape | Command::Deselect if !selection.is_empty() => {
                self.selection.set_empty();
                cx.redraw(self.id());
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
            Command::Left if pos > 0 => {
                let mut cursor = GraphemeCursor::new(pos, len, true);
                cursor
                    .prev_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            Command::Right | Command::End if !shift && have_sel => {
                Action::Move(selection.end, None)
            }
            Command::Right if pos < len => {
                let mut cursor = GraphemeCursor::new(pos, len, true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            Command::WordLeft if pos > 0 => {
                let mut iter = self.text.text()[0..pos].split_word_bound_indices();
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
            Command::WordRight if pos < len => {
                let mut iter = self.text.text()[pos..].split_word_bound_indices().skip(1);
                let mut p = iter.next().map(|(index, _)| pos + index).unwrap_or(len);
                while self.text.text()[p..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
                {
                    if let Some((index, _)) = iter.next() {
                        p = pos + index;
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
                        .text_glyph_pos(pos)?
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = self.text.find_line(pos)?.map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                line = match key {
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
                    .map(|pos| Action::Move(pos, Some(x)))
                    .unwrap_or(Action::Move(nearest_end, None))
            }
            Command::Home if pos > 0 => {
                let pos = self.text.find_line(pos)?.map(|r| r.1.start).unwrap_or(0);
                Action::Move(pos, None)
            }
            Command::End if pos < len => {
                let pos = self.text.find_line(pos)?.map(|r| r.1.end).unwrap_or(len);
                Action::Move(pos, None)
            }
            Command::DocHome if pos > 0 => Action::Move(0, None),
            Command::DocEnd if pos < len => Action::Move(len, None),
            Command::PageUp | Command::PageDown if multi_line => {
                let mut v = self
                    .text
                    .text_glyph_pos(pos)?
                    .next_back()
                    .map(|r| r.pos.into())
                    .unwrap_or(Vec2::ZERO);
                if let Some(x) = self.edit_x_coord {
                    v.0 = x;
                }
                const FACTOR: f32 = 2.0 / 3.0;
                let mut h_dist = self.text.env().bounds.1 * FACTOR;
                if key == Command::PageUp {
                    h_dist *= -1.0;
                }
                v.1 += h_dist;
                Action::Move(self.text.text_index_nearest(v.into())?, Some(v.0))
            }
            Command::Delete | Command::DelBack if editable && have_sel => {
                Action::Delete(selection.clone())
            }
            Command::Delete if editable => {
                let mut cursor = GraphemeCursor::new(pos, len, true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|next| Action::Delete(pos..next))
                    .unwrap_or(Action::None)
            }
            Command::DelBack if editable => {
                // We always delete one code-point, not one grapheme cluster:
                let prev = self.text.text()[0..pos]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                Action::Delete(prev..pos)
            }
            Command::DelWord if editable => {
                let next = self.text.text()[pos..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| pos + index)
                    .unwrap_or(len);
                Action::Delete(pos..next)
            }
            Command::DelWordBack if editable => {
                let prev = self.text.text()[0..pos]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Delete(prev..pos)
            }
            Command::SelectAll => {
                self.selection.set_sel_pos(0);
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
                if let Some((state, pos2, sel_pos)) = self.old_state.as_mut() {
                    self.text.swap_string(state);
                    self.selection.set_edit_pos(*pos2);
                    *pos2 = pos;
                    let pos = *sel_pos;
                    *sel_pos = self.selection.sel_pos();
                    self.selection.set_sel_pos(pos);
                    self.edit_x_coord = None;
                    self.last_edit = LastEdit::None;
                }
                Action::Edit
            }
            _ => Action::Unused,
        };

        let result = match action {
            Action::None => EditAction::None,
            Action::Unused => EditAction::Unused,
            Action::Activate => EditAction::Activate,
            Action::Edit => EditAction::Edit,
            Action::Insert(s, edit) => {
                let mut pos = pos;
                if have_sel {
                    self.old_state =
                        Some((self.text.clone_string(), pos, self.selection.sel_pos()));
                    self.last_edit = edit;

                    self.text.replace_range(selection.clone(), s);
                    pos = selection.start;
                } else {
                    if self.last_edit != edit {
                        self.old_state =
                            Some((self.text.clone_string(), pos, self.selection.sel_pos()));
                        self.last_edit = edit;
                    }

                    self.text.replace_range(pos..pos, s);
                }
                self.selection.set_pos(pos + s.len());
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Delete(sel) => {
                if self.last_edit != LastEdit::Delete {
                    self.old_state =
                        Some((self.text.clone_string(), pos, self.selection.sel_pos()));
                    self.last_edit = LastEdit::Delete;
                }

                self.text.replace_range(sel.clone(), "");
                self.selection.set_pos(sel.start);
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Move(pos, x_coord) => {
                self.selection.set_edit_pos(pos);
                if !shift {
                    self.selection.set_empty();
                } else {
                    self.set_primary(cx);
                }
                self.edit_x_coord = x_coord;
                cx.redraw(self.id());
                EditAction::None
            }
        };

        self.prepare_text(cx);
        Ok(result)
    }

    fn set_edit_pos_from_coord(&mut self, cx: &mut EventCx, coord: Coord) {
        let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
        if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
            if pos != self.selection.edit_pos() {
                self.selection.set_edit_pos(pos);
                self.set_view_offset_from_edit_pos(cx);
                self.edit_x_coord = None;
                cx.redraw(self.id());
            }
        }
    }

    fn set_primary(&self, cx: &mut EventCx) {
        if !self.selection.is_empty() {
            let range = self.selection.range();
            cx.set_primary(String::from(&self.text.as_str()[range]));
        }
    }

    // Pan by given delta. Return `Response::Scrolled` or `Response::Pan(remaining)`.
    fn pan_delta(&mut self, cx: &mut EventCx, mut delta: Offset) -> Response {
        let new_offset = (self.view_offset - delta)
            .min(self.max_scroll_offset())
            .max(Offset::ZERO);
        if new_offset != self.view_offset {
            delta -= self.view_offset - new_offset;
            self.view_offset = new_offset;
            cx.redraw(self.id());
        }

        cx.set_scroll(if delta == Offset::ZERO {
            Scroll::Scrolled
        } else {
            Scroll::Offset(delta)
        });
        Response::Used
    }

    /// Update view_offset after edit_pos changes
    ///
    /// A redraw is assumed since edit_pos moved.
    fn set_view_offset_from_edit_pos(&mut self, cx: &mut EventCx) {
        let edit_pos = self.selection.edit_pos();
        if let Some(marker) = self
            .text
            .text_glyph_pos(edit_pos)
            .ok()
            .and_then(|mut m| m.next_back())
        {
            let bounds = Vec2::from(self.text.env().bounds);
            let min_x = marker.pos.0 - bounds.0;
            let min_y = marker.pos.1 - marker.descent - bounds.1;
            let max_x = marker.pos.0;
            let max_y = marker.pos.1 - marker.ascent;
            let min = Offset(min_x.cast_ceil(), min_y.cast_ceil());
            let max = Offset(max_x.cast_floor(), max_y.cast_floor());

            let max = max.min(self.max_scroll_offset());

            let new_offset = self.view_offset.max(min).min(max);
            if new_offset != self.view_offset {
                self.view_offset = new_offset;
                cx.set_scroll(Scroll::Scrolled);
            }
        }
    }
}
