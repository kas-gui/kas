// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};
use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

use kas::class::HasString;
use kas::draw::{DrawHandleExt, TextClass};
use kas::event::{ControlKey, GrabMode};
use kas::prelude::*;

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
    Activate,
    Edit,
}

/// An [`EditBox`] with no [`EditGuard`]
///
/// This may be useful when requiring a fully-typed [`EditBox`]. Alternatively,
/// one may implement an [`EditGuard`], `G`, and use `EditBox<G>`.
pub type EditBoxVoid = EditBox<EditVoid>;

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
    /// The [`event::Handler::Msg`] type
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
    /// This function is called on any edit of the contents, by the user or
    /// programmatically. It is also called when the `EditGuard` is first set.
    /// On programmatic edits and the initial call, the return value of this
    /// method is discarded.
    fn edit(_: &mut EditBox<Self>) -> Option<Self::Msg> {
        None
    }
}

/// No-action [`EditGuard`]
#[derive(Clone, Debug)]
pub struct EditVoid;
impl EditGuard for EditVoid {
    type Msg = VoidMsg;
}

/// An [`EditGuard`] impl which calls a closure when activated
pub struct EditActivate<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditActivate<F, M> {
    type Msg = M;
    fn activate(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text())
    }
}

/// An [`EditGuard`] impl which calls a closure when activated or focus is lost
pub struct EditAFL<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditAFL<F, M> {
    type Msg = M;
    fn activate(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text())
    }
    fn focus_lost(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text())
    }
}

/// An [`EditGuard`] impl which calls a closure when edited
pub struct EditEdit<F: Fn(&str) -> Option<M>, M>(pub F);
impl<F: Fn(&str) -> Option<M>, M> EditGuard for EditEdit<F, M> {
    type Msg = M;
    fn edit(edit: &mut EditBox<Self>) -> Option<Self::Msg> {
        (edit.guard.0)(edit.text.text())
    }
}

/// An editable, single-line text box.
///
/// This widget is intended for use with short input strings. Internally it
/// uses a [`String`], for which edits have `O(n)` cost.
///
/// Currently, this widget has a [`EditBox::multi_line`] mode, with some
/// limitations (incorrect positioning of the edit cursor at line end,
/// non-functional up/down keys, lack of scrolling). Later this will be replaced
/// by a dedicated multi-line widget, probably using the `ropey` crate.
#[widget(config(key_nav = true, cursor_icon = event::CursorIcon::Text))]
#[handler(handle=noauto, generics = <> where G: EditGuard)]
#[derive(Clone, Default, Widget)]
pub struct EditBox<G: 'static> {
    #[widget_core]
    core: CoreData,
    frame_offset: Coord,
    frame_size: Size,
    text_pos: Coord,
    editable: bool,
    multi_line: bool,
    text: PreparedText,
    edit_pos: usize,
    sel_pos: usize,
    edit_x_coord: Option<f32>,
    old_state: Option<(String, usize, usize)>,
    last_edit: LastEdit,
    error_state: bool,
    /// The associated [`EditGuard`] implementation
    pub guard: G,
}

impl<G> Debug for EditBox<G> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EditBox {{ core: {:?}, editable: {:?}, text: {:?}, ... }}",
            self.core,
            self.editable,
            self.text.text()
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
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), frame_size, margins);

        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let content_rules = size_handle.text_bound(&mut self.text, class, axis);
        let m = content_rules.margins();

        let rules = content_rules.surrounded_by(frame_rules, true);
        if axis.is_horizontal() {
            self.core.rect.size.0 = rules.ideal_size();
            self.frame_offset.0 = frame_offset.0 as i32 + m.0 as i32;
            self.frame_size.0 = frame_size.0 + (m.0 + m.1) as u32;
        } else {
            self.core.rect.size.1 = rules.ideal_size();
            self.frame_offset.1 = frame_offset.1 as i32 + m.0 as i32;
            self.frame_size.1 = frame_size.1 + (m.0 + m.1) as u32;
        }
        rules
    }

    fn set_rect(&mut self, rect: Rect, align: AlignHints) {
        let valign = if self.multi_line {
            Align::Stretch
        } else {
            Align::Centre
        };
        let rect = align
            .complete(Align::Stretch, valign, self.rect().size)
            .apply(rect);

        self.core.rect = rect;
        self.text_pos = rect.pos + self.frame_offset;
        let size = rect.size - self.frame_size;
        let multi_line = self.multi_line;
        self.text.update_env(|env| {
            env.set_bounds(size.into());
            env.set_wrap(multi_line);
        });
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let mut input_state = self.input_state(mgr, disabled);
        input_state.error = self.error_state;
        draw_handle.edit_box(self.core.rect, input_state);
        if self.sel_pos == self.edit_pos {
            draw_handle.text(self.text_pos, &self.text, class);
        } else {
            draw_handle.text_selected(self.text_pos, &self.text, self.selection(), class);
        }
        if input_state.char_focus {
            draw_handle.edit_marker(self.text_pos, &self.text, class, self.edit_pos);
        }
    }
}

impl EditBox<EditVoid> {
    /// Construct an `EditBox` with the given inital `text`.
    pub fn new<S: Into<String>>(text: S) -> Self {
        let text = text.into();
        let edit_pos = text.len();
        EditBox {
            core: Default::default(),
            frame_offset: Default::default(),
            frame_size: Default::default(),
            text_pos: Default::default(),
            editable: true,
            multi_line: false,
            text: PreparedText::new(text.into()),
            edit_pos,
            sel_pos: edit_pos,
            edit_x_coord: None,
            old_state: None,
            last_edit: LastEdit::None,
            error_state: false,
            guard: EditVoid,
        }
    }

    /// Set an [`EditGuard`]
    ///
    /// Technically, this consumes `self` and reconstructs another `EditBox`
    /// with a different parameterisation.
    ///
    /// This method calls [`EditGuard::edit`] after applying `guard` to `self`
    /// and discards any message emitted.
    pub fn with_guard<G: EditGuard>(self, guard: G) -> EditBox<G> {
        let mut edit = EditBox {
            core: self.core,
            frame_offset: self.frame_offset,
            frame_size: self.frame_size,
            text_pos: self.text_pos,
            editable: self.editable,
            multi_line: self.multi_line,
            text: self.text,
            edit_pos: self.edit_pos,
            sel_pos: self.sel_pos,
            edit_x_coord: self.edit_x_coord,
            old_state: self.old_state,
            last_edit: self.last_edit,
            error_state: self.error_state,
            guard,
        };
        let _ = G::edit(&mut edit);
        edit
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
    /// The closure `f` is called when the `EditBox` is edited by the user.
    /// Its result, if not `None`, is the event handler's response.
    ///
    /// The closure `f` is also called initially (by this method) and on
    /// programmatic edits, however in these cases any results returned by `f`
    /// are discarded.
    ///
    /// This method is a parametisation of [`EditBox::with_guard`]. Any guard
    /// previously assigned to the `EditBox` will be replaced.
    pub fn on_edit<F: Fn(&str) -> Option<M>, M>(self, f: F) -> EditBox<EditEdit<F, M>> {
        self.with_guard(EditEdit(f))
    }
}

impl<G> EditBox<G> {
    /// Set whether this `EditBox` is editable (inline)
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Get whether this `EditBox` is editable
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether this `EditBox` is editable
    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// Set whether this `EditBox` shows multiple text lines
    pub fn multi_line(mut self, multi_line: bool) -> Self {
        self.multi_line = multi_line;
        self
    }

    /// Get whether the input state is erroneous
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

    fn selection(&self) -> Range<usize> {
        let mut range = self.edit_pos..self.sel_pos;
        if range.start > range.end {
            std::mem::swap(&mut range.start, &mut range.end);
        }
        range
    }

    fn received_char(&mut self, mgr: &mut Manager, c: char) -> EditAction {
        if !self.editable {
            return EditAction::None;
        }

        let pos = self.edit_pos;
        let selection = self.selection();
        let have_sel = selection.start < selection.end;
        if self.last_edit != LastEdit::Insert || have_sel {
            self.old_state = Some((self.text.clone_string(), pos, self.sel_pos));
            self.last_edit = LastEdit::Insert;
        }
        if have_sel {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            *mgr += self.text.replace_range(selection.clone(), s);
            self.edit_pos = selection.start + s.len();
        } else {
            *mgr += self.text.insert_char(pos, c);
            self.edit_pos = pos + c.len_utf8();
        }
        self.sel_pos = self.edit_pos;
        self.edit_x_coord = None;
        EditAction::Edit
    }

    fn control_key(&mut self, mgr: &mut Manager, key: ControlKey) -> EditAction {
        if !self.editable {
            return EditAction::None;
        }

        let mut mgr_action = TkAction::None;
        let mut buf = [0u8; 4];
        let pos = self.edit_pos;
        let selection = self.selection();
        let have_sel = selection.end > selection.start;
        let ctrl = mgr.modifiers().ctrl();
        let shift = mgr.modifiers().shift();
        let string;

        enum Action<'a> {
            None,
            Activate,
            Edit,
            Insert(&'a str, LastEdit),
            Delete(Range<usize>),
            Move(usize, Option<f32>),
        }

        let action = match key {
            ControlKey::Return if shift || !self.multi_line => Action::Activate,
            ControlKey::Return if self.multi_line => {
                Action::Insert('\n'.encode_utf8(&mut buf), LastEdit::Insert)
            }
            ControlKey::Home if ctrl => Action::Move(0, None),
            ControlKey::Home => {
                let pos = self.text.find_line(pos).map(|r| r.1.start).unwrap_or(0);
                Action::Move(pos, None)
            }
            ControlKey::End if ctrl => Action::Move(self.text.text_len(), None),
            ControlKey::End => {
                let pos = self
                    .text
                    .find_line(pos)
                    .map(|r| r.1.end)
                    .unwrap_or(self.text.text_len());
                Action::Move(pos, None)
            }
            ControlKey::Left if ctrl => {
                // TODO: This should find the next word-start, not *all* word
                // boundaries! Perhaps best to implement in kas-text since
                // anything external will struggle with bidirectional text.
                let pos = self.text.text()[0..pos]
                    .split_word_bound_indices()
                    .next_back()
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                Action::Move(pos, None)
            }
            ControlKey::Left => {
                let mut cursor = GraphemeCursor::new(pos, self.text.text_len(), true);
                cursor
                    .prev_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            ControlKey::Right if ctrl => {
                let pos = self.text.text()[pos..]
                    .split_word_bound_indices()
                    .skip(1)
                    .next()
                    .map(|(index, _)| pos + index)
                    .unwrap_or(self.text.text_len());
                Action::Move(pos, None)
            }
            ControlKey::Right => {
                let mut cursor = GraphemeCursor::new(pos, self.text.text_len(), true);
                cursor
                    .next_boundary(self.text.text(), 0)
                    .unwrap()
                    .map(|pos| Action::Move(pos, None))
                    .unwrap_or(Action::None)
            }
            ControlKey::Up | ControlKey::Down | ControlKey::PageUp | ControlKey::PageDown => {
                let x = match self.edit_x_coord {
                    Some(x) => x,
                    None => self
                        .text
                        .text_glyph_rel_pos(pos)
                        .next_back()
                        .map(|r| (r.0).0)
                        .unwrap_or(0.0),
                };
                let mut line = self.text.find_line(pos).map(|r| r.0).unwrap_or(0);
                // We can tolerate invalid line numbers here!
                // TODO: PageUp/Down should depend on view size?
                line = match key {
                    ControlKey::Up => line.wrapping_sub(1),
                    ControlKey::Down => line.wrapping_add(1),
                    ControlKey::PageUp => line.wrapping_sub(30),
                    ControlKey::PageDown => line.wrapping_add(30),
                    _ => unreachable!(),
                };
                const HALF: usize = usize::MAX / 2;
                let nearest_end = || match line {
                    0..=HALF => self.text.text_len(),
                    _ => 0,
                };
                self.text
                    .line_index_nearest(line, x)
                    .map(|pos| Action::Move(pos, Some(x)))
                    .unwrap_or(Action::Move(nearest_end(), None))
            }
            ControlKey::Delete => {
                if have_sel {
                    Action::Delete(selection.clone())
                } else if ctrl {
                    let next = self.text.text()[pos..]
                        .split_word_bound_indices()
                        .skip(1)
                        .next()
                        .map(|(index, _)| pos + index)
                        .unwrap_or(self.text.text_len());
                    Action::Delete(pos..next)
                } else {
                    let mut cursor = GraphemeCursor::new(pos, self.text.text_len(), true);
                    cursor
                        .next_boundary(self.text.text(), 0)
                        .unwrap()
                        .map(|next| Action::Delete(pos..next))
                        .unwrap_or(Action::None)
                }
            }
            ControlKey::Backspace => {
                if have_sel {
                    Action::Delete(selection.clone())
                } else if ctrl {
                    let prev = self.text.text()[0..pos]
                        .split_word_bound_indices()
                        .next_back()
                        .map(|(index, _)| index)
                        .unwrap_or(0);
                    Action::Delete(prev..pos)
                } else {
                    // We always delete one code-point, not one grapheme cluster:
                    let prev = self.text.text()[0..pos]
                        .char_indices()
                        .rev()
                        .next()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    Action::Delete(prev..pos)
                }
            }
            ControlKey::Cut if have_sel => {
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::Delete(selection.clone())
            }
            ControlKey::Copy if have_sel => {
                mgr.set_clipboard((self.text.text()[selection.clone()]).into());
                Action::None
            }
            ControlKey::Paste => {
                if let Some(content) = mgr.get_clipboard() {
                    let mut end = content.len();
                    if !self.multi_line {
                        // We cut the content short on control characters and
                        // ignore them (preventing line-breaks and ignoring any
                        // actions such as recursive-paste).
                        for (i, c) in content.char_indices() {
                            if c < '\u{20}' || (c >= '\u{7f}' && c <= '\u{9f}') {
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
            ControlKey::Undo | ControlKey::Redo => {
                // TODO: maintain full edit history (externally?)
                // NOTE: undo *and* redo shortcuts map to this control char
                if let Some((state, pos2, sel_pos)) = self.old_state.as_mut() {
                    mgr_action += self.text.swap_string(state);
                    self.edit_pos = *pos2;
                    *pos2 = pos;
                    std::mem::swap(sel_pos, &mut self.sel_pos);
                    self.edit_x_coord = None;
                    self.last_edit = LastEdit::None;
                }
                Action::Edit
            }
            _ => Action::None,
        };

        let result = match action {
            Action::None => EditAction::None,
            Action::Activate => EditAction::Activate,
            Action::Edit => EditAction::Edit,
            Action::Insert(s, edit) => {
                let mut pos = pos;
                if have_sel {
                    self.old_state = Some((self.text.clone_string(), pos, self.sel_pos));
                    self.last_edit = edit;

                    mgr_action += self.text.replace_range(selection.clone(), s);
                    pos = selection.start;
                } else {
                    if self.last_edit != edit {
                        self.old_state = Some((self.text.clone_string(), pos, self.sel_pos));
                        self.last_edit = edit;
                    }

                    mgr_action += self.text.replace_range(pos..pos, s);
                }
                self.edit_pos = pos + s.len();
                self.sel_pos = self.edit_pos;
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Delete(sel) => {
                if self.last_edit != LastEdit::Delete {
                    self.old_state = Some((self.text.clone_string(), pos, self.sel_pos));
                    self.last_edit = LastEdit::Delete;
                }

                mgr_action += self.text.replace_range(sel.clone(), "");
                self.edit_pos = sel.start;
                self.sel_pos = sel.start;
                self.edit_x_coord = None;
                EditAction::Edit
            }
            Action::Move(pos, x_coord) => {
                self.edit_pos = pos;
                if !shift {
                    self.sel_pos = self.edit_pos;
                }
                self.edit_x_coord = x_coord;
                mgr_action += TkAction::Redraw;
                EditAction::None
            }
        };

        *mgr += mgr_action;
        result
    }

    fn set_edit_pos_from_coord(&mut self, mgr: &mut Manager, coord: Coord) {
        self.edit_pos = self.text.text_index_nearest(self.text_pos, coord);
        self.edit_x_coord = None;
        mgr.redraw(self.id());
    }
}

impl<G: EditGuard> HasString for EditBox<G> {
    fn get_str(&self) -> &str {
        self.text.text()
    }

    fn set_string(&mut self, text: String) -> TkAction {
        let action = self.text.set_text(text);
        let _ = G::edit(self);
        action
    }
}

impl<G: EditGuard + 'static> event::Handler for EditBox<G> {
    type Msg = G::Msg;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<Self::Msg> {
        match event {
            Event::Activate => {
                mgr.request_char_focus(self.id());
                Response::None
            }
            Event::LostCharFocus => {
                let r = G::focus_lost(self);
                r.map(|msg| msg.into()).unwrap_or(Response::None)
            }
            Event::Control(key) => match self.control_key(mgr, key) {
                EditAction::None => Response::None,
                EditAction::Activate => G::activate(self).into(),
                EditAction::Edit => G::edit(self).into(),
            },
            Event::ReceivedCharacter(c) => match self.received_char(mgr, c) {
                EditAction::None => Response::None,
                EditAction::Activate => G::activate(self).into(),
                EditAction::Edit => G::edit(self).into(),
            },
            Event::PressStart { source, coord, .. } if source.is_primary() => {
                self.set_edit_pos_from_coord(mgr, coord);
                if !mgr.modifiers().shift() {
                    self.sel_pos = self.edit_pos;
                }
                mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                mgr.request_char_focus(self.id());
                Response::None
            }
            Event::PressMove { coord, .. } => {
                self.set_edit_pos_from_coord(mgr, coord);
                Response::None
            }
            Event::PressEnd { .. } => Response::None,
            event => Response::Unhandled(event),
        }
    }
}
