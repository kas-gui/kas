// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Text widgets

use std::fmt::{self, Debug};

use crate::class::{Editable, HasText};
use crate::event::{Action, CursorIcon, Handler, Manager, ManagerState, Response, VoidMsg};
use crate::layout::{AxisInfo, SizeRules};
use crate::macros::Widget;
use crate::theme::{DrawHandle, SizeHandle, TextClass, TextProperties};
use crate::{Align, AlignHints, CoreData, Layout, Widget, WidgetCore};
use kas::geom::Rect;

/// A simple text label
#[widget]
#[handler]
#[derive(Clone, Default, Debug, Widget)]
pub struct Label {
    #[core]
    core: CoreData,
    halign: Align,
    valign: Align,
    text: String,
}

impl Layout for Label {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let rules = size_handle.text_bound(&self.text, TextClass::Label, axis);
        if axis.is_horizontal() {
            self.core_data_mut().rect.size.0 = rules.ideal_size();
        } else {
            self.core_data_mut().rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, _size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        self.halign = align.horiz.unwrap_or(Align::Begin);
        self.valign = align.vert.unwrap_or(Align::Centre);
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, _: &ManagerState) {
        let props = TextProperties {
            class: TextClass::Label,
            horiz: self.halign,
            vert: self.valign,
        };
        draw_handle.text(self.core.rect, &self.text, props);
    }
}

impl Label {
    /// Construct a new, empty instance
    pub fn new<T: ToString>(text: T) -> Self {
        Label {
            core: Default::default(),
            halign: Default::default(),
            valign: Default::default(),
            text: text.to_string(),
        }
    }
}

impl<T> From<T> for Label
where
    String: From<T>,
{
    fn from(text: T) -> Self {
        Label {
            core: Default::default(),
            halign: Default::default(),
            valign: Default::default(),
            text: String::from(text),
        }
    }
}

impl HasText for Label {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
        self.text = text;
        mgr.redraw(self.id());
    }
}

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

/// An editable, single-line text box.
#[derive(Clone, Default, Widget)]
pub struct EditBox<H: 'static> {
    #[core]
    core: CoreData,
    text_rect: Rect,
    editable: bool,
    multi_line: bool,
    text: String,
    old_state: Option<String>,
    last_edit: LastEdit,
    on_activate: H,
}

impl<H> Debug for EditBox<H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "EditBox {{ core: {:?}, editable: {:?}, text: {:?}, ... }}",
            self.core, self.editable, self.text
        )
    }
}

impl<H: 'static> Widget for EditBox<H> {
    fn allow_focus(&self) -> bool {
        true
    }

    fn cursor_icon(&self) -> CursorIcon {
        CursorIcon::Text
    }
}

impl<H: 'static> Layout for EditBox<H> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let sides = size_handle.edit_surround();
        let margin = size_handle.inner_margin();
        let rules = SizeRules::fixed(axis.extract_size(sides.0 + sides.1 + margin))
            + size_handle.text_bound(&self.text, class, axis);
        if axis.is_horizontal() {
            self.core_data_mut().rect.size.0 = rules.ideal_size();
        } else {
            self.core_data_mut().rect.size.1 = rules.ideal_size();
        }
        rules
    }

    fn set_rect(&mut self, size_handle: &mut dyn SizeHandle, rect: Rect, align: AlignHints) {
        let valign = if self.multi_line {
            Align::Stretch
        } else {
            Align::Centre
        };
        let rect = align
            .complete(Align::Stretch, valign, self.rect().size)
            .apply(rect);

        let sides = size_handle.edit_surround();
        self.text_rect = Rect {
            pos: rect.pos + sides.0,
            size: rect.size - (sides.0 + sides.1),
        };
        self.core_data_mut().rect = rect;
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &ManagerState) {
        let class = if self.multi_line {
            TextClass::EditMulti
        } else {
            TextClass::Edit
        };
        let highlights = mgr.highlight_state(self.id());
        draw_handle.edit_box(self.core.rect, highlights);
        let props = TextProperties {
            class,
            horiz: Align::Begin,
            vert: Align::Begin,
        };
        let mut text = &self.text;
        let mut _string;
        if highlights.char_focus {
            _string = self.text.clone();
            _string.push('|');
            text = &_string;
        }
        draw_handle.text(self.text_rect, text, props);
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
            on_activate: (),
        }
    }

    /// Set the event handler to be called on activation.
    ///
    /// The closure `f` is called when the `EditBox` is activated (when the
    /// "enter" key is pressed). Its result is returned from the event handler.
    ///
    /// Technically, this consumes `self` and reconstructs another `EditBox`
    /// with a different parameterisation.
    pub fn on_activate<R, H: Fn(&str) -> R>(self, f: H) -> EditBox<H> {
        EditBox {
            core: self.core,
            text_rect: self.text_rect,
            editable: self.editable,
            multi_line: self.multi_line,
            text: self.text,
            old_state: self.old_state,
            last_edit: self.last_edit,
            on_activate: f,
        }
    }
}

impl<H> EditBox<H> {
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

    fn received_char(&mut self, mgr: &mut Manager, c: char) -> bool {
        if !self.editable {
            return false;
        }

        // TODO: Text selection and editing (see Unicode std. section 5.11)
        // Note that it may make sense to implement text shaping first.
        // For now we just filter control characters and append the rest.
        if c < '\u{20}' || (c >= '\u{7f}' && c <= '\u{9f}') {
            match c {
                '\u{03}' /* copy */ => {
                    // we don't yet have selection support, so just copy everything
                    mgr.set_clipboard(self.text.clone());
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
                '\u{0D}' /* carriage return (\r) */ => return true,
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
        false
    }
}

impl<H> HasText for EditBox<H> {
    fn get_text(&self) -> &str {
        &self.text
    }

    fn set_string(&mut self, mgr: &mut Manager, text: String) {
        self.text = text;
        mgr.redraw(self.id());
    }
}

impl<H> Editable for EditBox<H> {
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

impl<M, H: Fn(&str) -> M> Handler for EditBox<H> {
    type Msg = M;

    #[inline]
    fn activation_via_press(&self) -> bool {
        true
    }

    fn handle_action(&mut self, mgr: &mut Manager, action: Action) -> Response<M> {
        match action {
            Action::Activate => {
                mgr.request_char_focus(self.id());
                Response::None
            }
            Action::ReceivedCharacter(c) => {
                if self.received_char(mgr, c) {
                    ((self.on_activate)(&self.text)).into()
                } else {
                    Response::None
                }
            }
            a @ _ => Response::unhandled_action(a),
        }
    }
}
