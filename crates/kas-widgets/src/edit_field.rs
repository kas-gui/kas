// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text-edit field

use super::Scrollable;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{self, Command, Scroll, ScrollDelta};
use kas::geom::Vec2;
use kas::layout::{self, FrameStorage};
use kas::prelude::*;
use kas::text::SelectionHelper;
use kas::theme::{Background, FrameStyle, IdRect, TextClass};
use std::fmt::Debug;
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

#[derive(Clone, Debug, PartialEq)]
enum LastEdit {
    None,
    Insert,
    Delete,
    Paste,
}

impl Default for LastEdit {
    fn default() -> Self {
        LastEdit::None
    }
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
/// [`EditField`] and may return a message via [`EventMgr::push_msg`].
///
/// All methods on this trait are passed a reference to the [`EditField`] as
/// parameter. The `EditGuard`'s state may be accessed via the
/// [`EditField::guard`] public field.
///
/// All methods have a default implementation which does nothing.
///
/// This trait is implemented for `()` (does nothing).
pub trait EditGuard: Debug + Sized + 'static {
    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes.
    ///
    /// The default implementation does nothing.
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        let _ = (edit, mgr);
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

/// An [`EditGuard`] impl which calls a closure when activated
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct EditActivate<F: FnMut(&str, &mut EventMgr)>(pub F);
impl<F> EditGuard for EditActivate<F>
where
    F: FnMut(&str, &mut EventMgr) + 'static,
{
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(edit.text.text(), mgr);
    }
}

/// An [`EditGuard`] impl which calls a closure when activated or focus is lost
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct EditAFL<F: FnMut(&str, &mut EventMgr)>(pub F);
impl<F> EditGuard for EditAFL<F>
where
    F: FnMut(&str, &mut EventMgr) + 'static,
{
    fn activate(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(edit.text.text(), mgr);
    }
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(edit.text.text(), mgr);
    }
}

/// An [`EditGuard`] impl which calls a closure when edited
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct EditEdit<F: FnMut(&str, &mut EventMgr)>(pub F);
impl<F> EditGuard for EditEdit<F>
where
    F: FnMut(&str, &mut EventMgr) + 'static,
{
    fn edit(edit: &mut EditField<Self>, mgr: &mut EventMgr) {
        (edit.guard.0)(edit.text.text(), mgr);
    }
}

/// An [`EditGuard`] impl which calls a closure when updated
#[autoimpl(Debug ignore self.0)]
#[derive(Clone)]
pub struct EditUpdate<F: FnMut(&str)>(pub F);
impl<F: FnMut(&str) + 'static> EditGuard for EditUpdate<F> {
    fn update(edit: &mut EditField<Self>) {
        (edit.guard.0)(edit.text.text());
    }
}

impl_scope! {
    /// A text-edit box
    ///
    /// This is just a wrapper around [`EditField`] adding a frame.
    #[autoimpl(Deref, DerefMut, HasStr, HasString using self.inner)]
    #[derive(Clone, Default, Debug)]
    #[widget]
    pub struct EditBox<G: EditGuard = ()> {
        #[widget_core]
        core: CoreData,
        #[widget]
        inner: EditField<G>,
        frame_storage: FrameStorage,
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let inner = layout::Layout::single(&mut self.inner);
            layout::Layout::frame(&mut self.frame_storage, inner, FrameStyle::EditBox)
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let bg = if self.inner.has_error() {
                Background::Error
            } else {
                Background::Default
            };
            draw.frame(IdRect(self.inner.id_ref(), self.rect()), FrameStyle::EditBox, bg);
            self.inner.draw(draw.re());
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
            frame_storage: Default::default(),
        }
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
            frame_storage: self.frame_storage,
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
    pub fn on_activate<F>(self, f: F) -> EditBox<EditActivate<F>>
    where
        F: FnMut(&str, &mut EventMgr) + 'static,
    {
        self.with_guard(EditActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_afl<F>(self, f: F) -> EditBox<EditAFL<F>>
    where
        F: FnMut(&str, &mut EventMgr) + 'static,
    {
        self.with_guard(EditAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditBox` is edited by the user.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_edit<F>(self, f: F) -> EditBox<EditEdit<F>>
    where
        F: FnMut(&str, &mut EventMgr) + 'static,
    {
        self.with_guard(EditEdit(f))
    }

    /// Set a guard function, called on update
    ///
    /// The closure `f` is called when the `EditBox` is updated (by the user or
    /// programmatically). It is also called immediately by this method.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    #[must_use]
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditBox<EditUpdate<F>> {
        self.with_guard(EditUpdate(f))
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

    /// Get whether this widget is editable
    #[inline]
    pub fn is_editable(&self) -> bool {
        self.inner.is_editable()
    }

    /// Set whether this widget is editable
    #[inline]
    pub fn set_editable(&mut self, editable: bool) {
        self.inner.set_editable(editable);
    }

    /// Set whether this `EditBox` shows multiple text lines
    #[inline]
    #[must_use]
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.inner = self.inner.multi_line(multi_line);
        self
    }

    /// Get whether the widget currently has keyboard input focus
    #[inline]
    pub fn has_key_focus(&self) -> bool {
        self.inner.has_key_focus()
    }

    /// Get whether the input state is erroneous
    #[inline]
    pub fn has_error(&self) -> bool {
        self.inner.has_error()
    }

    /// Set the error state
    ///
    /// When true, the input field's background is drawn red.
    // TODO: possibly change type to Option<String> and display the error
    #[inline]
    pub fn set_error_state(&mut self, error_state: bool) {
        self.inner.set_error_state(error_state);
    }
}

impl_scope! {
    /// A text-edit field (single- or multi-line)
    ///
    /// Usually one uses a derived type like [`EditBox`] instead. This field does
    /// not draw any background or borders, thus (1) there is no visual indication
    /// that this is an edit field, and (2) there is no indication for disabled or
    /// error states. The parent widget is responsible for this.
    ///
    /// This widget is intended for use with short input strings. Internally it
    /// uses a [`String`], for which edits have `O(n)` cost.
    ///
    /// Optionally, [`EditField::multi_line`] mode can be activated (enabling
    /// line-wrapping and a larger vertical height). This mode is only recommended
    /// for short texts for performance reasons.
    #[derive(Clone, Default, Debug)]
    #[widget{
        key_nav = true;
        hover_highlight = true;
        cursor_icon = event::CursorIcon::Text;
    }]
    pub struct EditField<G: EditGuard = ()> {
        #[widget_core]
        core: CoreData,
        view_offset: Offset,
        editable: bool,
        multi_line: bool,
        text: Text<String>,
        required: Vec2,
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
            let class = TextClass::Edit(self.multi_line);
            size_mgr.text_bound(&mut self.text, class, axis)
        }

        fn set_rect(&mut self, mgr: &mut SetRectMgr, rect: Rect, align: AlignHints) {
            let valign = if self.multi_line {
                Align::Default
            } else {
                Align::Center
            };
            let class = TextClass::Edit(self.multi_line);

            self.core.rect = rect;
            let align = align.unwrap_or(Align::Default, valign);
            self.required = mgr.text_set_size(&mut self.text, class, rect.size, align);
            self.set_view_offset_from_edit_pos();
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn draw(&mut self, mut draw: DrawMgr) {
            let class = TextClass::Edit(self.multi_line);
            draw.with_clip_region(self.rect(), self.view_offset, |mut draw| {
                if self.selection.is_empty() {
                    draw.text(&*self, self.text.as_ref(), class);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(
                        &*self,
                        &self.text,
                        self.selection.range(),
                        class,
                    );
                }
                if self.editable && draw.ev_state().has_char_focus(self.id_ref()).0 {
                    draw.text_cursor(
                        &*self,
                        self.text.as_ref(),
                        class,
                        self.selection.edit_pos(),
                    );
                }
            });
        }
    }

    impl HasStr for Self {
        fn get_str(&self) -> &str {
            self.text.text()
        }
    }

    impl HasString for Self {
        fn set_string(&mut self, string: String) -> TkAction {
            // TODO: make text.set_string report bool for is changed?
            if *self.text.text() == string {
                return TkAction::empty();
            }

            self.text.set_string(string);
            self.selection.clear();
            if kas::text::fonts::fonts().num_faces() > 0 {
                if let Some(req) = self.text.prepare() {
                    self.required = req.into();
                }
            }
            G::update(self);
            TkAction::REDRAW
        }
    }

    impl event::Handler for Self
    where
        G: 'static,
    {
        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            fn request_focus<G: EditGuard + 'static>(s: &mut EditField<G>, mgr: &mut EventMgr) {
                if !s.has_key_focus && mgr.request_char_focus(s.id()) {
                    s.has_key_focus = true;
                    mgr.set_scroll(Scroll::Rect(s.rect()));
                    G::focus_gained(s, mgr);
                }
            }
            match event {
                Event::Activate | Event::NavFocus(true) => {
                    request_focus(self, mgr);
                    Response::Used
                }
                Event::NavFocus(false) => Response::Used,
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
                Event::Command(cmd, shift) => {
                    // Note: we can receive a Command without char focus, but should
                    // ensure we have focus before acting on it.
                    request_focus(self, mgr);
                    if self.has_key_focus {
                        match self.control_key(mgr, cmd, shift) {
                            EditAction::None => Response::Used,
                            EditAction::Unused => Response::Unused,
                            EditAction::Activate => {
                                G::activate(self, mgr);
                                Response::Used
                            }
                            EditAction::Edit => {
                                G::edit(self, mgr);
                                Response::Used
                            }
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
                        ScrollDelta::LineDelta(x, y) => {
                            // We arbitrarily scroll 3 lines:
                            let dist = 3.0 * self.text.env().height(Default::default());
                            Offset((x * dist).cast_nearest(), (y * dist).cast_nearest())
                        }
                        ScrollDelta::PixelDelta(coord) => coord,
                    };
                    self.pan_delta(mgr, delta2)
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
            let bounds = Vec2::from(self.text.env().bounds);
            let max_offset = Offset::conv_ceil(self.required - bounds);
            max_offset.max(Offset::ZERO)
        }

        fn scroll_offset(&self) -> Offset {
            self.view_offset
        }

        fn set_scroll_offset(&mut self, mgr: &mut EventMgr, offset: Offset) -> Offset {
            let new_offset = offset.min(self.max_scroll_offset()).max(Offset::ZERO);
            if new_offset != self.view_offset {
                self.view_offset = new_offset;
                // No widget moves so do not need to report TkAction::REGION_MOVED
                mgr.redraw(self.id());
            }
            new_offset
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
            core: Default::default(),
            view_offset: Default::default(),
            editable: true,
            multi_line: false,
            text: Text::new(Default::default(), text),
            required: Vec2::ZERO,
            selection: SelectionHelper::new(len, len),
            edit_x_coord: None,
            old_state: None,
            last_edit: LastEdit::None,
            has_key_focus: false,
            error_state: false,
            input_handler: Default::default(),
            guard: (),
        }
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
            multi_line: self.multi_line,
            text: self.text,
            required: self.required,
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
    pub fn on_activate<F: FnMut(&str, &mut EventMgr) + 'static>(
        self,
        f: F,
    ) -> EditField<EditActivate<F>> {
        self.with_guard(EditActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditField` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_afl<F: FnMut(&str, &mut EventMgr) + 'static>(self, f: F) -> EditField<EditAFL<F>> {
        self.with_guard(EditAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditField` is edited by the user.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_edit<F: FnMut(&str, &mut EventMgr) + 'static>(self, f: F) -> EditField<EditEdit<F>> {
        self.with_guard(EditEdit(f))
    }

    /// Set a guard function, called on update
    ///
    /// The closure `f` is called when the `EditField` is updated (by the user or
    /// programmatically). It is also called immediately by this method.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    #[must_use]
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditField<EditUpdate<F>> {
        self.with_guard(EditUpdate(f))
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

    /// Set whether this `EditField` shows multiple text lines
    #[inline]
    #[must_use]
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = multi_line;
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
            let _ = self.text.replace_range(selection.clone(), s);
            self.selection.set_pos(selection.start + s.len());
        } else {
            let _ = self.text.insert_char(pos, c);
            self.selection.set_pos(pos + c.len_utf8());
        }
        self.edit_x_coord = None;
        if let Some(req) = self.text.prepare() {
            self.required = req.into();
        }
        self.set_view_offset_from_edit_pos();
        mgr.redraw(self.id());
        true
    }

    fn control_key(&mut self, mgr: &mut EventMgr, key: Command, mut shift: bool) -> EditAction {
        if !self.editable {
            return EditAction::Unused;
        }

        let mut buf = [0u8; 4];
        let pos = self.selection.edit_pos();
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
            Command::Return if shift || !self.multi_line => Action::Activate,
            Command::Return if self.multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), LastEdit::Insert)
            }
            // NOTE: we might choose to optionally handle Tab in the future,
            // but without some workaround it prevents keyboard navigation.
            // Command::Tab => Action::Insert('\t'.encode_utf8(&mut buf), LastEdit::Insert),
            Command::Left => {
                let mut cursor = GraphemeCursor::new(pos, self.text.str_len(), true);
                cursor
                    .prev_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            Command::Right => {
                let mut cursor = GraphemeCursor::new(pos, self.text.str_len(), true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            Command::WordLeft => {
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
            Command::WordRight => {
                let mut iter = self.text.text()[pos..].split_word_bound_indices().skip(1);
                let mut p = iter
                    .next()
                    .map(|(index, _)| pos + index)
                    .unwrap_or(self.text.str_len());
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
            Command::Up | Command::Down => {
                let x = match self.edit_x_coord {
                    Some(x) => x,
                    None => self
                        .text
                        .text_glyph_pos(pos)
                        .next_back()
                        .map(|r| r.pos.0)
                        .unwrap_or(0.0),
                };
                let mut line = self.text.find_line(pos).map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                line = match key {
                    Command::Up => line.wrapping_sub(1),
                    Command::Down => line.wrapping_add(1),
                    _ => unreachable!(),
                };
                const HALF: usize = usize::MAX / 2;
                let nearest_end = || match line {
                    0..=HALF => self.text.str_len(),
                    _ => 0,
                };
                self.text
                    .line_index_nearest(line, x)
                    .map(|pos| Action::Move(pos, Some(x)))
                    .unwrap_or(Action::Move(nearest_end(), None))
            }
            Command::Home => {
                let pos = self.text.find_line(pos).map(|r| r.1.start).unwrap_or(0);
                Action::Move(pos, None)
            }
            Command::End => {
                let pos = self
                    .text
                    .find_line(pos)
                    .map(|r| r.1.end)
                    .unwrap_or(self.text.str_len());
                Action::Move(pos, None)
            }
            Command::DocHome => Action::Move(0, None),
            Command::DocEnd => Action::Move(self.text.str_len(), None),
            Command::PageUp | Command::PageDown => {
                let mut v = self
                    .text
                    .text_glyph_pos(pos)
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
                Action::Move(self.text.text_index_nearest(v.into()), Some(v.0))
            }
            Command::Delete | Command::DelBack if have_sel => Action::Delete(selection.clone()),
            Command::Delete => {
                let mut cursor = GraphemeCursor::new(pos, self.text.str_len(), true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|next| Action::Delete(pos..next))
                    .unwrap_or(Action::None)
            }
            Command::DelBack => {
                // We always delete one code-point, not one grapheme cluster:
                let prev = self.text.text()[0..pos]
                    .char_indices()
                    .rev()
                    .next()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                Action::Delete(prev..pos)
            }
            Command::DelWord => {
                let next = self.text.text()[pos..]
                    .split_word_bound_indices()
                    .nth(1)
                    .map(|(index, _)| pos + index)
                    .unwrap_or(self.text.str_len());
                Action::Delete(pos..next)
            }
            Command::DelWordBack => {
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
                Action::Move(self.text.str_len(), None)
            }
            Command::Cut if have_sel => {
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::Delete(selection.clone())
            }
            Command::Copy if have_sel => {
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::None
            }
            Command::Paste => {
                if let Some(content) = mgr.get_clipboard() {
                    let mut end = content.len();
                    if !self.multi_line {
                        // We cut the content short on control characters and
                        // ignore them (preventing line-breaks and ignoring any
                        // actions such as recursive-paste).
                        for (i, c) in content.char_indices() {
                            if c < '\u{20}' || ('\u{7f}'..='\u{9f}').contains(&c) {
                                end = i;
                                break;
                            }
                        }
                    }

                    string = content;
                    Action::Insert(&string[0..end], LastEdit::Paste)
                } else {
                    Action::None
                }
            }
            Command::Undo | Command::Redo => {
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
                }
                self.edit_x_coord = x_coord;
                mgr.redraw(self.id());
                EditAction::None
            }
        };

        let mut set_offset = self.selection.edit_pos() != pos;
        if !self.text.required_action().is_ready() {
            if let Some(req) = self.text.prepare() {
                self.required = req.into();
            }
            set_offset = true;
            mgr.redraw(self.id());
        }
        if set_offset {
            self.set_view_offset_from_edit_pos();
        }

        result
    }

    fn set_edit_pos_from_coord(&mut self, mgr: &mut EventMgr, coord: Coord) {
        let rel_pos = (coord - self.rect().pos + self.view_offset).cast();
        self.selection
            .set_edit_pos(self.text.text_index_nearest(rel_pos));
        self.set_view_offset_from_edit_pos();
        self.edit_x_coord = None;
        mgr.redraw(self.id());
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
    fn set_view_offset_from_edit_pos(&mut self) {
        let edit_pos = self.selection.edit_pos();
        if let Some(marker) = self.text.text_glyph_pos(edit_pos).next_back() {
            let bounds = Vec2::from(self.text.env().bounds);
            let min_x = marker.pos.0 - bounds.0;
            let min_y = marker.pos.1 - marker.descent - bounds.1;
            let max_x = marker.pos.0;
            let max_y = marker.pos.1 - marker.ascent;
            let min = Offset(min_x.cast_ceil(), min_y.cast_ceil());
            let max = Offset(max_x.cast_floor(), max_y.cast_floor());

            let max = max.min(self.max_scroll_offset());

            self.view_offset = self.view_offset.max(min).min(max);
        }
    }
}
