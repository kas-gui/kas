// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};

use crate::class::{Editable, HasText};
use crate::draw::{DrawHandle, SizeHandle, TextClass};
use crate::event::{self, Action, Handler, Manager, Response, VoidMsg};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::{Align, AlignHints, CoreData, CowString, Layout, WidgetCore};
use kas::geom::Rect;

#[derive(Clone, Debug, PartialEq)]
enum LastEdit {
    None,
    Insert,
    Backspace,
    Clear,
    Paste,
}

impl Default for LastEdit {
    fn default() -> Self {
        LastEdit::None
    }
}

enum EditAction {
    None,
    Activate,
    Edit,
}

/// A *guard* around an [`EditBox`]
///
/// When an [`EditBox`] receives input, it updates its contents as expected,
/// then invokes a method of `EditGuard`. This method may update the
/// [`EditBox`] and may return a message to be returned by the [`EditBox`]'s
/// event handler.
///
/// All methods on this trait are passed a reference to the [`EditBox`] as
/// parameter. The `EditGuard`'s state may be accessed via the
/// [`EditBox::guard`] public field.
///
/// All methods have a default implementation which does nothing.
pub trait EditGuard: Sized {
    /// The [`Handler::Msg`] type
    type Msg;

    /// Activation guard
    ///
    /// This function is called when the widget is "activated", for example by
    /// the Enter/Return key for single-line edit boxes.
    ///
    /// Note that activation events cannot edit the contents.
    fn activate(_: &mut EditBox<Self>) -> Option<Self::Msg> {
        None
    }

    /// Focus-lost guard
    ///
    /// This function is called when the widget loses keyboard input focus.
    fn focus_lost(_: &mut EditBox<Self>) -> Option<Self::Msg> {
        None
    }

    /// Edit guard
    ///
    /// This function is called on any edit of the contents.
    fn edit(_: &mut EditBox<Self>) -> Option<Self::Msg> {
        None
    }
}

/// A simple implementation of [`EditGuard`]
///
/// The wrapped closure is called with the [`EditBox`]'s contents whenever the
/// edit box is activated, and the response, if not `None`, is returned by the
/// event handler.
pub struct EditActivate<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditActivate<F, M> {
    type Msg = M;
    fn activate(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(&edit.text)
    }
}

pub struct EditAFL<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditAFL<F, M> {
    type Msg = M;
    fn activate(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(&edit.text)
    }
    fn focus_lost(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(&edit.text)
    }
}
pub struct EditEdit<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditEdit<F, M> {
    type Msg = M;
    fn edit(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(&edit.text)
    }
}

/// An editable, single-line text box.
#[widget]
#[widget_core(key_nav = true, cursor_icon = event::CursorIcon::Text)]
#[derive(Clone, Default, Widget)]
pub struct EditBox<G: 'static> {
    #[widget_core]
    core: CoreData,
    // During sizing, text_rect is used for the frame+inner-margin dimensions
    text_rect: Rect,
    editable: bool,
    multi_line: bool,
    text: String,
    old_state: Option<String>,
    last_edit: LastEdit,
    /// The associated [`EditGuard`] implementation
    pub guard: G,
}

impl<G> Debug for EditBox<G> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EditBox {{ core: {:?}, editable: {:?}, text: {:?}, ... }}",
            self.core, self.editable, self.text
        )
    }
}

impl<G: 'static> Layout for EditBox<G> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let frame_sides = size_handle.edit_surround();
        let inner = size_handle.inner_margin();
        let frame_offset = frame_sides.0 + inner;
        let frame_size = frame_offset + frame_sides.1 + inner;

        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.dir(), frame_size, margins);

        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let content_rules = size_handle.text_bound(&self.text, class, axis);
        let m = content_rules.margins();

        let rules = content_rules.surrounded_by(frame_rules, true);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
            self.text_rect.pos.0 = frame_offset.0 as i32 + m.0 as i32;
            self.text_rect.size.0 = frame_size.0 + (m.0 + m.1) as u32;
        } else {
            self.core.rect.size.1 = rules.ideal_size();
            self.text_rect.pos.1 = frame_offset.1 as i32 + m.0 as i32;
            self.text_rect.size.1 = frame_size.1 + (m.0 + m.1) as u32;
        }
        rules
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let valign = if self.multi_line {
            Align::Stretch
        } else {
            Align::Centre
        };
        let rect = align
            .complete(Align::Stretch, valign, self.rect().size)
            .apply(rect);

        self.core.rect = rect;
        self.text_rect.pos += rect.pos;
        self.text_rect.size = rect.size - self.text_rect.size;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState) {
        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let highlights = mgr.highlight_state(self.id());
        draw_handle.edit_box(self.core.rect, highlights);
        let align = (Align::Begin, Align::Begin);
        let mut text = &self.text;
        let mut _string;
        if highlights.char_focus {
            _string = self.text.clone();
            _string.push('|');
            text = &_string;
        }
        draw_handle.text(self.text_rect, text, class, align);
    }
}

impl EditBox<()> {
    /// Construct an `EditBox` with the given inital `text`.
    pub fn new<S: Into<String>>(text: S) -> Self {
        EditBox {
            core: Default::default(),
            text_rect: Default::default(),
            editable: true,
            multi_line: false,
            text: text.into(),
            old_state: None,
            last_edit: LastEdit::None,
            guard: (),
        }
    }

    /// Set an [`EditGuard`]
    ///
    /// Technically, this consumes `self` and reconstructs another `EditBox`
    /// with a different parameterisation.
    pub fn with_guard<G>(self, guard: G) -> EditBox<G> {
        EditBox {
            core: self.core,
            text_rect: self.text_rect,
            editable: self.editable,
            multi_line: self.multi_line,
            text: self.text,
            old_state: self.old_state,
            last_edit: self.last_edit,
            guard,
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
    pub fn on_activate<F: Fn(&str) -> Option<M>, M>(self, f: F) -> EditBox<EditActivate<F, M>> {
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
    pub fn on_afl<F: Fn(&str) -> Option<M>, M>(self, f: F) -> EditBox<EditAFL<F, M>> {
        self.with_guard(EditAFL(f))
    }

    /// Set a guard function, called on edit
    ///
    /// The closure `f` is called when the `EditBox` is edited.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    pub fn on_edit<F: Fn(&str) -> Option<M>, M>(self, f: F) -> EditBox<EditEdit<F, M>> {
        self.with_guard(EditEdit(f))
    }
}

impl<G> EditBox<G> {
    /// Set whether this `EditBox` is editable.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Set whether this `EditBox` shows multiple text lines
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = multi_line;
        self
    }

    fn received_char(&mut self, mgr: &mut Manager, c: char) -> EditAction {
        if !self.editable {
            return EditAction::None;
        }

        // TODO: Text selection and editing (see Unicode std. section 5.11)
        // Note that it may make sense to implement text shaping first.
        // For now we just filter control characters and append the rest.
        if c < '\u{20}' || (c >= '\u{7f}' && c <= '\u{9f}') {
            match c {
                '\u{03}' /* copy */ => {
                    // we don't yet have selection support, so just copy everything
                    mgr.set_clipboard((&self.text).into());
                }
                '\u{08}' /* backspace */  => {
                    if self.last_edit != LastEdit::Backspace {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Backspace;
                    }
                    self.text.pop();
                }
                '\u{09}' /* tab */ => (),
                '\u{0A}' /* line feed */ => (),
                '\u{0B}' /* vertical tab */ => (),
                '\u{0C}' /* form feed */ => (),
                '\u{0D}' /* carriage return (\r) */ => return EditAction::Activate,
                '\u{16}' /* paste */ => {
                    if self.last_edit != LastEdit::Paste {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Paste;
                    }
                    if let Some(content) = mgr.get_clipboard() {
                        // We cut the content short on control characters and
                        // ignore them (preventing line-breaks and ignoring any
                        // actions such as recursive-paste).
                        let mut end = content.len();
                        for (i, b) in content.as_bytes().iter().cloned().enumerate() {
                            if b < 0x20 || (b >= 0x7f && b <= 0x9f) {
                                end = i;
                                break;
                            }
                        }
                        self.text.push_str(&content[0..end]);
                    }
                }
                '\u{1A}' /* undo and redo */ => {
                    // TODO: maintain full edit history (externally?)
                    // NOTE: undo *and* redo shortcuts map to this control char
                    if let Some(state) = self.old_state.as_mut() {
                        std::mem::swap(state, &mut self.text);
                        self.last_edit = LastEdit::None;
                    }
                }
                '\u{1B}' /* escape */ => (),
                '\u{7f}' /* delete */ => {
                    if self.last_edit != LastEdit::Clear {
                        self.old_state = Some(self.text.clone());
                        self.last_edit = LastEdit::Clear;
                    }
                    self.text.clear();
                }
                _ => (),
            };
        } else {
            if self.last_edit != LastEdit::Insert {
                self.old_state = Some(self.text.clone());
                self.last_edit = LastEdit::Insert;
            }
            self.text.push(c);
        }
        mgr.redraw(self.id());
        EditAction::Edit
    }
}

impl<G> HasText for EditBox<G> {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_cow_string(&mut self, mgr: &mut Manager, text: CowString) {
        self.text = text.to_string();
        mgr.redraw(self.id());
    }
}

impl<G> Editable for EditBox<G> {
    fn is_editable(&self) -> bool {
        self.editable
    }

    fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }
}

impl Handler for EditBox<()> {
    type Msg = VoidMsg;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<VoidMsg> {
        match action {
            Action::Activate => {
                mgr.request_char_focus(self.id());
                Response::None
            }
            Action::ReceivedCharacter(c) => {
                self.received_char(mgr, c);
                Response::None
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}

impl<G: EditGuard + 'static> Handler for EditBox<G> {
    type Msg = G::Msg;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<Self::Msg> {
        match action {
            Action::Activate => {
                mgr.request_char_focus(self.id());
                Response::None
            }
            Action::LostCharFocus => {
                let r = G::focus_lost(self);
                r.map(|msg| msg.into()).unwrap_or(Response::None)
            }
            Action::ReceivedCharacter(c) => {
                let r = match self.received_char(mgr, c) {
                    EditAction::None => None,
                    EditAction::Activate => G::activate(self),
                    EditAction::Edit => G::edit(self),
                };
                r.map(|msg| msg.into()).unwrap_or(Response::None)
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}
