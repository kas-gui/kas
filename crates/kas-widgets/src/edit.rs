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
use kas::text::{NotReady, SelectionHelper};
use kas::theme::{Background, FrameStyle, TextClass};
use std::fmt::Debug;
use std::ops::Range;
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

/// A *guard* around an [`EditField`]
///
/// When an [`EditField`] receives input, it updates its contents as expected,
/// then invokes a method of `EditGuard`. This method may update the
/// [`EditField`] and may return a message via [`EventMgr::push`].
///
/// All methods on this trait are passed a reference to the [`EditField`] as
/// parameter. The `EditGuard`'s state may be accessed via the
/// [`EditField::guard`] public field.
///
/// All methods have a default implementation which does nothing.
///
/// Pre-built implementations:
///
/// -   `()`: does nothing
/// -   `GuardNotify`: clones text to a `String` and pushes as a message ([`EventMgr::push`])
///     on `activate` and `focus_lost` events
/// -   `GuardActivate: calls a closure on `activate`
/// -   `GuardAFL`: calls a closure on `activate` and `focus_lost`
/// -   `GuardEdit`: calls a closure on `edit`
/// -   `GuardUpdate`: calls a closure on `update`
pub trait EditGuard: Sized + 'static {
    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its result is returned
    /// from `handle_event`.
    ///
    /// The default implementation returns [`Response::Unused`].
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) -> Response {
        let _ = (edit, mgr);
        Response::Unused
    }

    /// Focus-gained guard
    ///
    /// This function is called when the widget gains keyboard input focus.
    fn focus_gained(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        let _ = (edit, mgr);
    }

    /// Focus-lost guard
    ///
    /// This function is called when the widget loses keyboard input focus.
    ///
    /// The default implementation does nothing.
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        let _ = (edit, mgr);
    }

    /// Edit guard
    ///
    /// This function is called when contents are updated by the user (but not
    /// on programmatic updates — see also [`EditGuard::update`]).
    ///
    /// The default implementation calls [`EditGuard::update`].
    fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        Self::update(edit);
        let _ = mgr;
    }

    /// Update guard
    ///
    /// This function is called on any programmatic update to the contents
    /// (and potentially also by [`EditGuard::edit`]).
    fn update(edit: &mut EditField<Self>) {
        let _ = edit;
    }
}

impl EditGuard for () {}

/// An [`EditGuard`] impl which notifies on activate and focus lost
///
/// On activate and focus-lost actions, calls [`EventMgr::push`] with the
/// edit's contents as a [`String`].
///
/// `Self::activate` returns [`Response::Used`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct GuardNotify;
impl EditGuard for GuardNotify {
    #[inline]
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) -> Response {
        Self::focus_lost(edit, mgr);
        Response::Used
    }

    #[inline]
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        mgr.push(edit.get_string());
    }
}

/// An [`EditGuard`] impl which calls a closure when activated
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct GuardActivate<F: FnMut(&mut EventMgr, &str) -> Response>(pub F);
impl<F> EditGuard for GuardActivate<F>
where
    F: FnMut(&mut EventMgr, &str) -> Response + 'static,
{
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) -> Response {
        (edit.guard.0)(mgr, edit.text.text())
    }
}

/// An [`EditGuard`] impl which calls a closure when activated or focus is lost
///
/// `Self::activate` returns [`Response::Used`].
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct GuardAFL<F: FnMut(&mut EventMgr, &str)>(pub F);
impl<F> EditGuard for GuardAFL<F>
where
    F: FnMut(&mut EventMgr, &str) + 'static,
{
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) -> Response {
        (edit.guard.0)(mgr, edit.text.text());
        Response::Used
    }
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(mgr, edit.text.text());
    }
}

/// An [`EditGuard`] impl which calls a closure when edited
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct GuardEdit<F: FnMut(&mut EventMgr, &str)>(pub F);
impl<F> EditGuard for GuardEdit<F>
where
    F: FnMut(&mut EventMgr, &str) + 'static,
{
    fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(mgr, edit.text.text());
    }
}

/// An [`EditGuard`] impl which calls a closure when updated
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct GuardUpdate<F: FnMut(&str)>(pub F);
impl<F: FnMut(&str) + 'static> EditGuard for GuardUpdate<F> {
    fn update(edit: &mut EditField<Self>) {
        (edit.guard.0)(edit.text.text());
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
    #[derive(Clone, Default, Debug)]
    #[widget]
    pub struct EditBox<G: EditGuard = ()> {
        core: widget_core!(),
        #[widget]
        inner: EditField<G>,
        #[widget]
        bar: ScrollBar<kas::dir::Down>,
        frame_offset: Offset,
        frame_size: Size,
        inner_margin: i32,
    }

    impl Layout for Self {
        fn size_rules(&mut self, mgr: SizeMgr, mut axis: AxisInfo) -> SizeRules {
            axis.sub_other(self.frame_size.extract(axis.flipped()));

            let mut rules = self.inner.size_rules(mgr.re(), axis);
            if axis.is_horizontal() && self.multi_line() {
                let bar_rules = self.bar.size_rules(mgr.re(), axis);
                self.inner_margin = rules.margins_i32().1.max(bar_rules.margins_i32().0);
                rules.append(bar_rules);
            }

            let frame_rules = mgr.frame(FrameStyle::EditBox, axis);
            let (rules, offset, size) = frame_rules.surround(rules);
            self.frame_offset.set_component(axis, offset);
            self.frame_size.set_component(axis, size);
            rules
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, mut rect: Rect) {
            self.core.rect = rect;
            rect.pos += self.frame_offset;
            rect.size -= self.frame_size;
            if self.multi_line() {
                let bar_width = mgr.size_mgr().scroll_bar_width();
                let x1 = rect.pos.0 + rect.size.0;
                let x0 = x1 - bar_width;
                let bar_rect = Rect::new(Coord(x0, rect.pos.1), Size(bar_width, rect.size.1));
                self.bar.set_rect(mgr, bar_rect);
                rect.size.0 = (rect.size.0 - bar_width - self.inner_margin).max(0);
            }
            self.inner.set_rect(mgr, rect);
            self.update_scroll_bar(mgr);
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

        fn draw(&mut self, mut draw: DrawMgr) {
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
        type Data = ();

        fn handle_message(&mut self, data: &Self::Data, mgr: &mut EventMgr<'_>) {
            if let Some(ScrollMsg(y)) = mgr.try_pop() {
                self.inner
                    .set_scroll_offset(data, mgr, Offset(self.inner.view_offset.0, y));
            }
        }

        fn handle_scroll(&mut self, _: &Self::Data, mgr: &mut EventMgr<'_>, _: Scroll) {
            self.update_scroll_bar(mgr);
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

        fn set_scroll_offset(&mut self, data: &Self::Data, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let offset = self.inner.set_scroll_offset(data, mgr, offset);
            self.update_scroll_bar(mgr);
            offset
        }
    }

    impl Self {
        fn update_scroll_bar(&mut self, mgr: &mut EventState) {
            let max_offset = self.inner.max_scroll_offset().1;
            *mgr |= self.bar.set_limits(max_offset, self.inner.rect().size.1);
            self.bar.set_value(mgr, self.inner.view_offset.1);
        }
    }
}

impl EditBox<()> {
    /// Construct an `EditBox` with the given inital `text`
    #[inline]
    pub fn new<S: ToString>(text: S) -> Self {
        EditBox {
            core: Default::default(),
            inner: EditField::new(text),
            bar: ScrollBar::new(),
            frame_offset: Offset::ZERO,
            frame_size: Size::ZERO,
            inner_margin: 0,
        }
    }

    /// Construct an empty `EditBox`
    #[inline]
    pub fn empty() -> Self {
        Self::new(String::new())
    }

    /// Set an [`EditGuard`]
    ///
    /// Technically, this consumes `self` and reconstructs another `EditBox`
    /// with a different parameterisation.
    ///
    /// This method calls [`EditGuard::update`] after applying `guard` to `self`
    /// and discards any message emitted.
    #[inline]
    #[must_use]
    pub fn with_guard<G: EditGuard>(self, guard: G) -> EditBox<G> {
        EditBox {
            core: self.core,
            inner: self.inner.with_guard(guard),
            bar: self.bar,
            frame_offset: self.frame_offset,
            frame_size: self.frame_size,
            inner_margin: self.inner_margin,
        }
    }

    /// Set a guard function, called on activation
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed).
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_activate<F>(self, f: F) -> EditBox<GuardActivate<F>>
    where
        F: FnMut(&mut EventMgr, &str) -> Response + 'static,
    {
        self.with_guard(GuardActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_afl<F>(self, f: F) -> EditBox<GuardAFL<F>>
    where
        F: FnMut(&mut EventMgr, &str) + 'static,
    {
        self.with_guard(GuardAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditBox` is edited by the user.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_edit<F>(self, f: F) -> EditBox<GuardEdit<F>>
    where
        F: FnMut(&mut EventMgr, &str) + 'static,
    {
        self.with_guard(GuardEdit(f))
    }

    /// Set a guard function, called on update
    ///
    /// The closure `f` is called when the `EditBox` is updated (by the user or
    /// programmatically). It is also called immediately by this method.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditBox<GuardUpdate<F>> {
        self.with_guard(GuardUpdate(f))
    }
}

impl<G: EditGuard> EditBox<G> {
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
    #[derive(Clone, Debug)]
    #[widget{
        navigable = true;
        hover_highlight = true;
        cursor_icon = CursorIcon::Text;
    }]
    pub struct EditField<G: EditGuard = ()> {
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
        fn size_rules(&mut self, size_mgr: SizeMgr, axis: AxisInfo) -> SizeRules {
            let (min, ideal) = if axis.is_horizontal() {
                let dpem = size_mgr.dpem();
                ((self.width.0 * dpem).cast_ceil(), (self.width.1 * dpem).cast_ceil())
            } else {
                let height = size_mgr.line_height(self.class);
                (self.lines.0 * height, self.lines.1 * height)
            };
            let margins = size_mgr.text_margins().extract(axis);
            let (stretch, align) = if axis.is_horizontal() || self.multi_line() {
                (Stretch::High, axis.align_or_default())
            } else {
                (Stretch::None, axis.align_or_center())
            };
            self.align.set_component(axis, align);
            SizeRules::new(min, ideal, margins, stretch)
        }

        fn set_rect(&mut self, mgr: &mut ConfigMgr, rect: Rect) {
            self.core.rect = rect;
            mgr.text_set_size(&mut self.text, self.class, rect.size, Some(self.align));
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();
            self.view_offset = self.view_offset.min(self.max_scroll_offset());
        }

        fn draw(&mut self, mut draw: DrawMgr) {
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
        type Data = ();

        fn handle_event(&mut self, _: &Self::Data, mgr: &mut EventMgr, event: Event) -> Response {
            fn request_focus<G: EditGuard + 'static>(s: &mut EditField<G>, mgr: &mut EventMgr) {
                if !s.has_key_focus && mgr.request_char_focus(s.id()) {
                    s.has_key_focus = true;
                    mgr.set_scroll(Scroll::Rect(s.rect()));
                    G::focus_gained(s, mgr);
                }
            }
            match event {
                Event::NavFocus(true) => {
                    request_focus(self, mgr);
                    if !self.class.multi_line() {
                        self.selection.clear();
                        self.selection.set_edit_pos(self.text.str_len());
                        mgr.redraw(self.id());
                    }
                    Response::Used
                }
                Event::NavFocus(false) => Response::Used,
                Event::LostNavFocus => {
                    if !self.class.multi_line() {
                        self.selection.set_empty();
                        mgr.redraw(self.id());
                    }
                    Response::Used
                }
                Event::LostCharFocus => {
                    self.has_key_focus = false;
                    mgr.redraw(self.id());
                    G::focus_lost(self, mgr);
                    Response::Used
                }
                Event::LostSelFocus => {
                    self.selection.set_empty();
                    mgr.redraw(self.id());
                    Response::Used
                }
                Event::Command(cmd) => {
                    // Note: we can receive a Command without char focus, but should
                    // ensure we have focus before acting on it.
                    request_focus(self, mgr);
                    if self.has_key_focus {
                        match self.control_key(mgr, cmd) {
                            Ok(EditAction::None) => Response::Used,
                            Ok(EditAction::Unused) => Response::Unused,
                            Ok(EditAction::Activate) => G::activate(self, mgr),
                            Ok(EditAction::Edit) => {
                                G::edit(self, mgr);
                                Response::Used
                            }
                            Err(NotReady) => Response::Used,
                        }
                    } else {
                        Response::Unused
                    }
                }
                Event::ReceivedCharacter(c) => match self.received_char(mgr, c) {
                    false => Response::Unused,
                    true => {
                        G::edit(self, mgr);
                        Response::Used
                    }
                },
                Event::Scroll(delta) => {
                    let delta2 = match delta {
                        ScrollDelta::LineDelta(x, y) => mgr.config().scroll_distance((x, y)),
                        ScrollDelta::PixelDelta(coord) => coord,
                    };
                    self.pan_delta(mgr, delta2)
                }
                Event::PressStart { press } if press.is_tertiary() =>
                    press.grab(self.id())
                        .with_mode(kas::event::GrabMode::Click)
                        .with_mgr(mgr),
                Event::PressEnd { press, .. } if press.is_tertiary() => {
                    if let Some(content) = mgr.get_primary() {
                        self.set_edit_pos_from_coord(mgr, press.coord);
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
                        self.prepare_text(mgr);

                        G::edit(self, mgr);
                    }
                    Response::Used
                }
                event => match self.input_handler.handle(mgr, self.id(), event) {
                    TextInputAction::None => Response::Used,
                    TextInputAction::Unused => Response::Unused,
                    TextInputAction::Pan(delta) => self.pan_delta(mgr, delta),
                    TextInputAction::Focus => {
                        request_focus(self, mgr);
                        Response::Used
                    }
                    TextInputAction::Cursor(coord, anchor, clear, repeats) => {
                        request_focus(self, mgr);
                        if self.has_key_focus {
                            self.set_edit_pos_from_coord(mgr, coord);
                            if anchor {
                                self.selection.set_anchor();
                            }
                            if clear {
                                self.selection.set_empty();
                            }
                            if repeats > 1 {
                                self.selection.expand(&self.text, repeats);
                            }
                            self.set_primary(mgr);
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

        fn set_scroll_offset(&mut self, _: &Self::Data, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.view_offset = new_offset;
                // No widget moves so do not need to report Action::REGION_MOVED
                mgr.redraw(self.id());
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
            G::update(self);
            action
        }
    }
}

impl EditField<()> {
    /// Construct an `EditField` with the given inital `text`
    #[inline]
    pub fn new<S: ToString>(text: S) -> Self {
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

    /// Construct an empty `EditField`
    #[inline]
    pub fn empty() -> Self {
        Self::new(String::new())
    }

    /// Set an [`EditGuard`]
    ///
    /// Technically, this consumes `self` and reconstructs another `EditField`
    /// with a different parameterisation.
    ///
    /// This method calls [`EditGuard::update`] after applying `guard` to `self`
    /// and discards any message emitted.
    #[inline]
    #[must_use]
    pub fn with_guard<G: EditGuard>(self, guard: G) -> EditField<G> {
        let mut edit = EditField {
            core: self.core,
            view_offset: self.view_offset,
            editable: self.editable,
            class: self.class,
            align: self.align,
            width: self.width,
            lines: self.lines,
            text: self.text,
            text_size: self.text_size,
            selection: self.selection,
            edit_x_coord: self.edit_x_coord,
            old_state: self.old_state,
            last_edit: self.last_edit,
            has_key_focus: self.has_key_focus,
            error_state: self.error_state,
            input_handler: self.input_handler,
            guard,
        };
        G::update(&mut edit);
        edit
    }

    /// Set a guard function, called on activation
    ///
    /// The closure `f` is called when the `EditField` is activated (when the
    /// "enter" key is pressed).
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_activate<F: FnMut(&mut EventMgr, &str) -> Response + 'static>(
        self,
        f: F,
    ) -> EditField<GuardActivate<F>> {
        self.with_guard(GuardActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditField` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_afl<F: FnMut(&mut EventMgr, &str) + 'static>(self, f: F) -> EditField<GuardAFL<F>> {
        self.with_guard(GuardAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditField` is edited by the user.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_edit<F: FnMut(&mut EventMgr, &str) + 'static>(self, f: F) -> EditField<GuardEdit<F>> {
        self.with_guard(GuardEdit(f))
    }

    /// Set a guard function, called on update
    ///
    /// The closure `f` is called when the `EditField` is updated (by the user or
    /// programmatically). It is also called immediately by this method.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditField<GuardUpdate<F>> {
        self.with_guard(GuardUpdate(f))
    }
}

impl<G: EditGuard> EditField<G> {
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
    // TODO: possibly change type to Option<String> and display the error
    pub fn set_error_state(&mut self, error_state: bool) {
        self.error_state = error_state;
    }

    fn prepare_text(&mut self, mgr: &mut EventMgr) {
        if !self.text.required_action().is_ready() {
            let start = std::time::Instant::now();

            self.text.prepare().expect("invalid font_id");
            self.text_size = Vec2::from(self.text.bounding_box().unwrap().1).cast_ceil();

            log::trace!(
                target: "kas_perf::widgets::edit", "prepare_text: {}μs",
                start.elapsed().as_micros(),
            );
        }

        mgr.redraw(self.id());
        self.set_view_offset_from_edit_pos(mgr);
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
    fn received_char(&mut self, mgr: &mut EventMgr, c: char) -> bool {
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

        self.prepare_text(mgr);
        true
    }

    fn control_key(&mut self, mgr: &mut EventMgr, key: Command) -> Result<EditAction, NotReady> {
        let editable = self.editable;
        let mut shift = mgr.modifiers().shift();
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
                mgr.redraw(self.id());
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
            Command::Left if !shift && have_sel => Action::Move(selection.start, None),
            Command::Left if pos > 0 => {
                let mut cursor = GraphemeCursor::new(pos, len, true);
                cursor
                    .prev_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            Command::Right if !shift && have_sel => Action::Move(selection.end, None),
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
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::Delete(selection.clone())
            }
            Command::Copy if have_sel => {
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::None
            }
            Command::Paste if editable => {
                if let Some(content) = mgr.get_clipboard() {
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
                    self.set_primary(mgr);
                }
                self.edit_x_coord = x_coord;
                mgr.redraw(self.id());
                EditAction::None
            }
        };

        self.prepare_text(mgr);
        Ok(result)
    }

    fn set_edit_pos_from_coord(&mut self, mgr: &mut EventMgr, coord: Coord) {
        let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
        if let Ok(pos) = self.text.text_index_nearest(rel_pos) {
            if pos != self.selection.edit_pos() {
                self.selection.set_edit_pos(pos);
                self.set_view_offset_from_edit_pos(mgr);
                self.edit_x_coord = None;
                mgr.redraw(self.id());
            }
        }
    }

    fn set_primary(&self, mgr: &mut EventMgr) {
        if !self.selection.is_empty() {
            let range = self.selection.range();
            mgr.set_primary(String::from(&self.text.as_str()[range]));
        }
    }

    // Pan by given delta. Return `Response::Scrolled` or `Response::Pan(remaining)`.
    fn pan_delta(&mut self, mgr: &mut EventMgr, mut delta: Offset) -> Response {
        let new_offset = (self.view_offset - delta)
            .min(self.max_scroll_offset())
            .max(Offset::ZERO);
        if new_offset != self.view_offset {
            delta -= self.view_offset - new_offset;
            self.view_offset = new_offset;
            mgr.redraw(self.id());
        }

        mgr.set_scroll(if delta == Offset::ZERO {
            Scroll::Scrolled
        } else {
            Scroll::Offset(delta)
        });
        Response::Used
    }

    /// Update view_offset after edit_pos changes
    ///
    /// A redraw is assumed since edit_pos moved.
    fn set_view_offset_from_edit_pos(&mut self, mgr: &mut EventMgr) {
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
                mgr.set_scroll(Scroll::Scrolled);
            }
        }
    }
}
