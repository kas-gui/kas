// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use super::Scrollable;
use kas::draw::TextClass;
use kas::event::components::{TextInput, TextInputAction};
use kas::event::{self, Command, ScrollDelta};
use kas::geom::Vec2;
use kas::layout;
use kas::prelude::*;
use kas::text::SelectionHelper;
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
    Unhandled,
    Activate,
    Edit,
}

/// A *guard* around an [`EditField`]
///
/// When an [`EditField`] receives input, it updates its contents as expected,
/// then invokes a method of `EditGuard`. This method may update the
/// [`EditField`] and may return a message to be returned by the [`EditField`]'s
/// event handler.
///
/// All methods on this trait are passed a reference to the [`EditField`] as
/// parameter. The `EditGuard`'s state may be accessed via the
/// [`EditField::guard`] public field.
///
/// All methods have a default implementation which does nothing.
///
/// This trait is implemented for `()` (does nothing; Msg = VoidMsg).
pub trait EditGuard: Debug + Sized + 'static {
    /// The [`event::Handler::Msg`] type
    type Msg;

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes. Its return value is
    /// converted to [`Response::None`] or [`Response::Msg`].
    ///
    /// Note that activation events cannot edit the contents.
    fn activate(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        let _ = (edit, mgr);
        None
    }

    /// Focus-gained guard
    ///
    /// This function is called when the widget gains keyboard input focus.
    fn focus_gained(edit: &mut EditField<Self>, mgr: &mut Manager) {
        let _ = (edit, mgr);
    }

    /// Focus-lost guard
    ///
    /// This function is called when the widget loses keyboard input focus. Its
    /// return value is converted to [`Response::None`] or [`Response::Msg`].
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        let _ = (edit, mgr);
        None
    }

    /// Edit guard
    ///
    /// This function is called when contents are updated by the user (but not
    /// on programmatic updates — see also [`EditGuard::update`]). Its return
    /// value is converted to [`Response::Update`] or [`Response::Msg`].
    ///
    /// The default implementation calls [`EditGuard::update`].
    fn edit(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        Self::update(edit);
        let _ = mgr;
        None
    }

    /// Update guard
    ///
    /// This function is called on any programmatic update to the contents
    /// (and potentially also by [`EditGuard::edit`]).
    fn update(edit: &mut EditField<Self>) {
        let _ = edit;
    }
}

impl EditGuard for () {
    type Msg = VoidMsg;
}

/// An [`EditGuard`] impl which calls a closure when activated
#[autoimpl(Debug skip 0)]
#[derive(Clone)]
pub struct EditActivate<F: FnMut(&str, &mut Manager) -> Option<M>, M>(pub F);
impl<F, M: 'static> EditGuard for EditActivate<F, M>
where
    F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
{
    type Msg = M;
    fn activate(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text(), mgr)
    }
}

/// An [`EditGuard`] impl which calls a closure when activated or focus is lost
#[autoimpl(Debug skip 0)]
#[derive(Clone)]
pub struct EditAFL<F: FnMut(&str, &mut Manager) -> Option<M>, M>(pub F);
impl<F, M: 'static> EditGuard for EditAFL<F, M>
where
    F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
{
    type Msg = M;
    fn activate(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text(), mgr)
    }
    fn focus_lost(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text(), mgr)
    }
}

/// An [`EditGuard`] impl which calls a closure when edited
#[autoimpl(Debug skip 0)]
#[derive(Clone)]
pub struct EditEdit<F: FnMut(&str, &mut Manager) -> Option<M>, M>(pub F);
impl<F, M: 'static> EditGuard for EditEdit<F, M>
where
    F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
{
    type Msg = M;
    fn edit(edit: &mut EditField<Self>, mgr: &mut Manager) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text(), mgr)
    }
}

/// An [`EditGuard`] impl which calls a closure when updated
#[autoimpl(Debug skip 0)]
#[derive(Clone)]
pub struct EditUpdate<F: FnMut(&str)>(pub F);
impl<F: FnMut(&str) + 'static> EditGuard for EditUpdate<F> {
    type Msg = VoidMsg;
    fn update(edit: &mut EditField<Self>) {
        (edit.guard.0)(edit.text.text());
    }
}

widget! {
    /// A text-edit box
    ///
    /// This is just a wrapper around [`EditField`] adding a frame.
    #[autoimpl(Deref, DerefMut, HasStr, HasString on inner)]
    #[derive(Clone, Default, Debug)]
    #[handler(msg = G::Msg)]
    pub struct EditBox<G: EditGuard = ()> {
        #[widget_core]
        core: CoreData,
        #[widget]
        inner: EditField<G>,
        layout_frame: layout::FrameStorage,
    }

    impl Layout for Self {
        fn layout(&mut self) -> layout::Layout<'_> {
            let inner = layout::Layout::single(&mut self.inner);
            layout::Layout::frame(&mut self.layout_frame, inner)
        }

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            // We draw highlights for input state of inner:
            let disabled = disabled || self.is_disabled() || self.inner.is_disabled();
            let mut input_state = self.inner.input_state(mgr, disabled);
            if self.inner.has_error() {
                input_state.insert(InputState::ERROR);
            }
            draw.edit_box(self.core.rect, input_state);
            self.inner.draw(draw, mgr, disabled);
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
            layout_frame: Default::default(),
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
    pub fn with_guard<G: EditGuard>(self, guard: G) -> EditBox<G> {
        EditBox {
            core: self.core,
            inner: self.inner.with_guard(guard),
            layout_frame: self.layout_frame,
        }
    }

    /// Set a guard function, called on activation
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed).
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    pub fn on_activate<F, M: 'static>(self, f: F) -> EditBox<EditActivate<F, M>>
    where
        F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
    {
        self.with_guard(EditActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    pub fn on_afl<F, M: 'static>(self, f: F) -> EditBox<EditAFL<F, M>>
    where
        F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
    {
        self.with_guard(EditAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditBox` is edited by the user.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    pub fn on_edit<F, M: 'static>(self, f: F) -> EditBox<EditEdit<F, M>>
    where
        F: FnMut(&str, &mut Manager) -> Option<M> + 'static,
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
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditBox<EditUpdate<F>> {
        self.with_guard(EditUpdate(f))
    }
}

impl<G: EditGuard> EditBox<G> {
    /// Set whether this `EditBox` is editable (inline)
    #[inline]
    pub fn editable(mut self, editable: bool) -> Self {
        self.inner = self.inner.editable(editable);
        self
    }

    /// Get whether this `EditBox` is editable
    #[inline]
    pub fn is_editable(&self) -> bool {
        self.inner.is_editable()
    }

    /// Set whether this `EditBox` is editable
    #[inline]
    pub fn set_editable(&mut self, editable: bool) {
        self.inner.set_editable(editable);
    }

    /// Set whether this `EditBox` shows multiple text lines
    #[inline]
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

widget! {
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
        fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
            let class = if self.multi_line {
                TextClass::EditMulti
            } else {
                TextClass::Edit
            };
            size_handle.text_bound(&mut self.text, class, axis)
        }

        fn set_rect(&mut self, _: &mut Manager, rect: Rect, align: AlignHints) {
            let valign = if self.multi_line {
                Align::Default
            } else {
                Align::Center
            };

            self.core.rect = rect;
            let size = rect.size;
            let multi_line = self.multi_line;
            self.required = self
                .text
                .update_env(|env| {
                    env.set_align(align.unwrap_or(Align::Default, valign));
                    env.set_bounds(size.into());
                    env.set_wrap(multi_line);
                })
                .into();
            self.set_view_offset_from_edit_pos();
        }

        #[inline]
        fn translation(&self) -> Offset {
            self.scroll_offset()
        }

        fn draw(&mut self, draw: &mut dyn DrawHandle, mgr: &ManagerState, disabled: bool) {
            let class = if self.multi_line {
                TextClass::EditMulti
            } else {
                TextClass::Edit
            };
            let state = self.input_state(mgr, disabled);
            draw.with_clip_region(self.rect(), self.view_offset, &mut |draw| {
                if self.selection.is_empty() {
                    draw.text(self.rect().pos, self.text.as_ref(), class, state);
                } else {
                    // TODO(opt): we could cache the selection rectangles here to make
                    // drawing more efficient (self.text.highlight_lines(range) output).
                    // The same applies to the edit marker below.
                    draw.text_selected(
                        self.rect().pos,
                        &self.text,
                        self.selection.range(),
                        class,
                        state,
                    );
                }
                if mgr.has_char_focus(self.id()).0 {
                    draw.edit_marker(
                        self.rect().pos,
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
            let _ = G::update(self);
            TkAction::REDRAW
        }
    }

    impl event::Handler for Self
    where
        G: 'static,
    {
        type Msg = G::Msg;

        #[inline]
        fn focus_on_key_nav(&self) -> bool {
            false
        }

        fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
            fn request_focus<G: EditGuard + 'static>(s: &mut EditField<G>, mgr: &mut Manager) {
                if !s.has_key_focus && mgr.request_char_focus(s.id()) {
                    s.has_key_focus = true;
                    G::focus_gained(s, mgr);
                }
            }
            match event {
                Event::Activate | Event::NavFocus(true) => {
                    request_focus(self, mgr);
                    Response::Focus(self.rect())
                }
                Event::NavFocus(false) => Response::None,
                Event::LostCharFocus => {
                    self.has_key_focus = false;
                    mgr.redraw(self.id());
                    G::focus_lost(self, mgr)
                        .map(|msg| msg.into())
                        .unwrap_or(Response::None)
                }
                Event::LostSelFocus => {
                    self.selection.set_empty();
                    mgr.redraw(self.id());
                    Response::None
                }
                Event::Command(cmd, shift) => {
                    // Note: we can receive a Command without char focus, but should
                    // ensure we have focus before acting on it.
                    request_focus(self, mgr);
                    if self.has_key_focus {
                        match self.control_key(mgr, cmd, shift) {
                            EditAction::None => Response::None,
                            EditAction::Unhandled => Response::Unhandled,
                            EditAction::Activate => Response::none_or_msg(G::activate(self, mgr)),
                            EditAction::Edit => Response::update_or_msg(G::edit(self, mgr)),
                        }
                    } else {
                        Response::Unhandled
                    }
                }
                Event::ReceivedCharacter(c) => match self.received_char(mgr, c) {
                    false => Response::Unhandled,
                    true => Response::update_or_msg(G::edit(self, mgr)),
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
                    match self.pan_delta(mgr, delta2) {
                        delta if delta == Offset::ZERO => Response::None,
                        delta => Response::Pan(delta),
                    }
                }
                event => match self.input_handler.handle(mgr, self.id(), event) {
                    TextInputAction::None => Response::None,
                    TextInputAction::Unhandled => Response::Unhandled,
                    TextInputAction::Pan(delta) => match self.pan_delta(mgr, delta) {
                        delta if delta == Offset::ZERO => Response::None,
                        delta => Response::Pan(delta),
                    },
                    TextInputAction::Focus => {
                        request_focus(self, mgr);
                        Response::None
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
                        Response::None
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
            let max_offset = (self.required - bounds).ceil();
            Offset::from(max_offset).max(Offset::ZERO)
        }

        fn scroll_offset(&self) -> Offset {
            self.view_offset
        }

        fn set_scroll_offset(&mut self, mgr: &mut Manager, offset: Offset) -> Offset {
            let new_offset = offset.clamp(Offset::ZERO, self.max_scroll_offset());
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
        let _ = G::update(&mut edit);
        edit
    }

    /// Set a guard function, called on activation
    ///
    /// The closure `f` is called when the `EditField` is activated (when the
    /// "enter" key is pressed).
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    pub fn on_activate<F: FnMut(&str, &mut Manager) -> Option<M> + 'static, M: 'static>(
        self,
        f: F,
    ) -> EditField<EditActivate<F, M>> {
        self.with_guard(EditActivate(f))
    }

    /// Set a guard function, called on activation and input-focus lost
    ///
    /// The closure `f` is called when the `EditField` is activated (when the
    /// "enter" key is pressed) and when keyboard focus is lost.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    pub fn on_afl<F: FnMut(&str, &mut Manager) -> Option<M> + 'static, M: 'static>(
        self,
        f: F,
    ) -> EditField<EditAFL<F, M>> {
        self.with_guard(EditAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditField` is edited by the user.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    pub fn on_edit<F: FnMut(&str, &mut Manager) -> Option<M> + 'static, M: 'static>(
        self,
        f: F,
    ) -> EditField<EditEdit<F, M>> {
        self.with_guard(EditEdit(f))
    }

    /// Set a guard function, called on update
    ///
    /// The closure `f` is called when the `EditField` is updated (by the user or
    /// programmatically). It is also called immediately by this method.
    ///
    /// This method is a parametisation of [`EditField::with_guard`]. Any guard
    /// previously assigned to the `EditField` will be replaced.
    pub fn on_update<F: FnMut(&str) + 'static>(self, f: F) -> EditField<EditUpdate<F>> {
        self.with_guard(EditUpdate(f))
    }
}

impl<G: EditGuard> EditField<G> {
    /// Set whether this `EditField` is editable (inline)
    #[inline]
    pub fn editable(mut self, editable: bool) -> Self {
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
    fn received_char(&mut self, mgr: &mut Manager, c: char) -> bool {
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

    fn control_key(&mut self, mgr: &mut Manager, key: Command, mut shift: bool) -> EditAction {
        if !self.editable {
            return EditAction::Unhandled;
        }

        let mut buf = [0u8; 4];
        let pos = self.selection.edit_pos();
        let selection = self.selection.range();
        let have_sel = selection.end > selection.start;
        let string;

        enum Action<'a> {
            None,
            Unhandled,
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
            _ => Action::Unhandled,
        };

        let result = match action {
            Action::None => EditAction::None,
            Action::Unhandled => EditAction::Unhandled,
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

    fn set_edit_pos_from_coord(&mut self, mgr: &mut Manager, coord: Coord) {
        let rel_pos = (coord - self.rect().pos + self.view_offset).into();
        self.selection
            .set_edit_pos(self.text.text_index_nearest(rel_pos));
        self.set_view_offset_from_edit_pos();
        self.edit_x_coord = None;
        mgr.redraw(self.id());
    }

    // Pan by given delta. Return remaining (unused) delta.
    fn pan_delta(&mut self, mgr: &mut Manager, delta: Offset) -> Offset {
        let new_offset = (self.view_offset - delta).clamp(Offset::ZERO, self.max_scroll_offset());
        if new_offset != self.view_offset {
            let delta = delta - (self.view_offset - new_offset);
            self.view_offset = new_offset;
            mgr.redraw(self.id());
            delta
        } else {
            delta
        }
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
