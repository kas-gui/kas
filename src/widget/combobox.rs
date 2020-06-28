// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use std::fmt::Debug;
use std::iter::FromIterator;

use super::{Column, MenuEntry, MenuFrame};
use kas::class::{HasRichText, SetText};
use kas::draw::TextClass;
use kas::event::{ControlKey, GrabMode};
use kas::prelude::*;
use kas::WindowId;

/// A pop-up multiple choice menu
#[widget(config(key_nav = true))]
#[handler(noauto)]
#[derive(Clone, Debug, Widget)]
pub struct ComboBox<M: Clone + Debug + 'static> {
    #[widget_core]
    core: CoreData,
    label: PreparedText,
    #[widget]
    popup: ComboPopup,
    messages: Vec<M>, // TODO: is this a useless lookup step?
    active: usize,
    opening: bool,
    popup_id: Option<WindowId>,
}

impl<M: Clone + Debug + 'static> kas::Layout for ComboBox<M> {
    fn size_rules(&mut self, size_handle: &mut dyn SizeHandle, axis: AxisInfo) -> SizeRules {
        let sides = size_handle.button_surround();
        let margins = size_handle.outer_margins();
        let frame_rules = SizeRules::extract_fixed(axis.is_vertical(), sides.0 + sides.1, margins);

        let content_rules = size_handle.text_bound(&mut self.label, TextClass::Button, axis);
        content_rules.surrounded_by(frame_rules, true)
    }

    fn set_rect(&mut self, rect: Rect, align: kas::AlignHints) {
        self.core.rect = rect;
        self.label.set_size(rect.size.into());
        self.label.set_alignment(
            align.horiz.unwrap_or(Align::Centre),
            align.vert.unwrap_or(Align::Centre),
        );
    }

    fn spatial_range(&self) -> (usize, usize) {
        // We have no child within our rect; return an empty range
        (0, std::usize::MAX)
    }

    fn draw(&self, draw_handle: &mut dyn DrawHandle, mgr: &event::ManagerState, disabled: bool) {
        let mut state = self.input_state(mgr, disabled);
        if self.popup_id.is_some() {
            state.depress = true;
        }
        draw_handle.button(self.core.rect, state);
        draw_handle.text(self.core.rect.pos, &self.label, TextClass::Button);
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    /// Construct a combobox
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    /// Each choice has some corresponding message of type `M` which is emitted
    /// by the event handler when this choice is selected.
    ///
    /// This constructor may be used with an iterator compatible with any
    /// [`FromIterator`] for `ComboBox`, for example:
    /// ```
    /// # use kas::widget::ComboBox;
    /// let combobox = ComboBox::<i32>::new([("one", 1), ("two", 2), ("three", 3)].iter());
    /// ```
    #[inline]
    pub fn new<T, I: IntoIterator<Item = T>>(iter: I) -> Self
    where
        ComboBox<M>: FromIterator<T>,
    {
        ComboBox::from_iter(iter)
    }

    #[inline]
    fn new_(column: Vec<MenuEntry<u64>>, messages: Vec<M>) -> Self {
        assert!(column.len() > 0, "ComboBox: expected at least one choice");
        let label = PreparedText::new(column[0].clone_rich_text(), false);
        ComboBox {
            core: Default::default(),
            label,
            popup: ComboPopup {
                core: Default::default(),
                inner: MenuFrame::new(Column::new(column)),
            },
            messages,
            active: 0,
            opening: false,
            popup_id: None,
        }
    }

    /// Get the index of the active choice
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active choice
    ///
    /// Panics if `index >= self.len()`.
    #[inline]
    pub fn set_active(&mut self, index: usize) -> TkAction {
        if index >= self.messages.len() {
            panic!("ComboBox::set_active(index): index out of bounds");
        }
        if self.active != index {
            self.active = index;
            self.label
                .set_text(self.popup.inner.inner[self.active].clone_rich_text())
        } else {
            TkAction::None
        }
    }

    /// Get the message associated with the active choice
    #[inline]
    pub fn msg(&self) -> M {
        self.messages[self.active].clone()
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    /// Get the number of entries
    #[inline]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Add a choice to the combobox, in last position
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn push<T: Into<CowString>>(&mut self, label: T, msg: M) -> TkAction {
        self.messages.push(msg);
        let column = &mut self.popup.inner.inner;
        let len = column.len() as u64;
        column.push(MenuEntry::new(label.into(), len))
        // TODO: localised reconfigure
    }

    // TODO: fn pop() -> Option<..> — but API is invalid if we can't remove last item

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn insert<T: Into<CowString>>(&mut self, index: usize, label: T, msg: M) -> TkAction {
        self.messages.insert(index, msg);
        let column = &mut self.popup.inner.inner;
        let len = column.len() as u64;
        column.insert(index, MenuEntry::new(label.into(), len))
        // TODO: localised reconfigure
    }

    /// Removes the choice at position `index` and returns its message
    ///
    /// Panics if `index` is out of bounds or if the removal would leave the
    /// `ComboBox` empty (which is not allowed).
    ///
    /// Triggers a [reconfigure action](Manager::send_action).
    pub fn remove(&mut self, index: usize) -> (M, TkAction) {
        if self.messages.len() < 2 {
            panic!("ComboBox::remove: unable to remove last choice");
        }
        let m = self.messages.remove(index);
        (m, self.popup.inner.inner.remove(index).1)
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<CowString>>(&mut self, index: usize, label: T, msg: M) -> (M, TkAction) {
        let mut m = msg;
        std::mem::swap(&mut m, &mut self.messages[index]);
        (m, self.popup.inner.inner[index].set_text(label))
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    fn map_response(&mut self, mgr: &mut Manager, r: Response<u64>) -> Response<M> {
        match r {
            Response::None => Response::None,
            Response::Unhandled(ev) => match ev {
                Event::Control(key, modifiers) => {
                    let next = |mgr: &mut Manager, s, clr, rev| {
                        if clr {
                            mgr.clear_nav_focus();
                        }
                        mgr.next_nav_focus(s, rev);
                        Response::None
                    };
                    match key {
                        ControlKey::Up => next(mgr, self, false, true),
                        ControlKey::Down => next(mgr, self, false, false),
                        ControlKey::Home => next(mgr, self, true, false),
                        ControlKey::End => next(mgr, self, true, true),
                        key => Response::Unhandled(Event::Control(key, modifiers)),
                    }
                }
                ev => Response::Unhandled(ev),
            },
            Response::Focus(x) => Response::Focus(x),
            Response::Msg(msg) => {
                let index = msg as usize;
                *mgr += self.set_active(index);
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                }
                Response::Msg(self.messages[index].clone())
            }
        }
        // NOTE: as part of the Popup API we are expected to trap
        // TkAction::Close here, but we know our widget doesn't generate
        // this action.
    }
}

impl<T: Into<AccelString>, M: Clone + Debug> FromIterator<(T, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = (T, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or(0);
        let mut choices = Vec::with_capacity(len);
        let mut messages = Vec::with_capacity(len);
        for (i, (label, msg)) in iter.enumerate() {
            choices.push(MenuEntry::new(label, i as u64));
            messages.push(msg);
        }
        ComboBox::new_(choices, messages)
    }
}

impl<'a, M: Clone + Debug + 'static> FromIterator<&'a (&'static str, M)> for ComboBox<M> {
    fn from_iter<I: IntoIterator<Item = &'a (&'static str, M)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or(0);
        let mut choices = Vec::with_capacity(len);
        let mut messages = Vec::with_capacity(len);
        for (i, (label, msg)) in iter.enumerate() {
            choices.push(MenuEntry::new(*label, i as u64));
            messages.push(msg.clone());
        }
        ComboBox::new_(choices, messages)
    }
}

impl<M: Clone + Debug + 'static> event::Handler for ComboBox<M> {
    type Msg = M;

    fn handle(&mut self, mgr: &mut Manager, event: Event) -> Response<M> {
        let open_popup = |s: &mut Self, mgr: &mut Manager| {
            let id = mgr.add_popup(kas::Popup {
                id: s.popup.id(),
                parent: s.id(),
                direction: Direction::Down,
            });
            s.popup_id = Some(id);
            if let Some(id) = s.popup.inner.inner.get(s.active).map(|w| w.id()) {
                mgr.set_nav_focus(id);
            }
        };
        match event {
            Event::Activate => {
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                } else {
                    open_popup(self, mgr);
                }
            }
            Event::PressStart {
                source,
                start_id,
                coord,
            } => {
                if self.is_ancestor_of(start_id) {
                    if source.is_primary() {
                        mgr.request_grab(self.id(), source, coord, GrabMode::Grab, None);
                        mgr.set_grab_depress(source, Some(start_id));
                        self.opening = self.popup_id.is_none();
                    }
                } else {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                    }
                    return Response::Unhandled(Event::None);
                }
            }
            Event::PressMove {
                source,
                cur_id,
                coord,
                ..
            } => {
                if self.popup_id.is_none() {
                    open_popup(self, mgr);
                }
                let cond = self.popup.inner.inner.rect().contains(coord);
                let target = if cond { cur_id } else { None };
                mgr.set_grab_depress(source, target);
                if let Some(id) = target {
                    mgr.set_nav_focus(id);
                }
            }
            Event::PressEnd { end_id, .. } => {
                if let Some(id) = end_id {
                    if id == self.id() {
                        if self.opening {
                            if self.popup_id.is_none() {
                                open_popup(self, mgr);
                            }
                            return Response::None;
                        }
                    } else if self.popup_id.is_some() && self.popup.is_ancestor_of(id) {
                        let r = self.popup.send(mgr, id, Event::Activate);
                        return self.map_response(mgr, r);
                    }
                }
                if let Some(id) = self.popup_id {
                    mgr.close_window(id);
                }
            }
            Event::NewPopup(id) => {
                // For a ComboBox, for any new Popup we should close self
                if id != self.popup.id() {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id);
                    }
                }
            }
            Event::PopupRemoved(id) => {
                debug_assert_eq!(Some(id), self.popup_id);
                self.popup_id = None;
            }
            event => return Response::Unhandled(event),
        }
        Response::None
    }
}

impl<M: Clone + Debug + 'static> event::SendEvent for ComboBox<M> {
    fn send(&mut self, mgr: &mut Manager, id: WidgetId, event: Event) -> Response<Self::Msg> {
        if self.is_disabled() {
            return Response::Unhandled(event);
        }

        if id <= self.popup.id() {
            let r = self.popup.send(mgr, id, event);
            self.map_response(mgr, r)
        } else {
            Manager::handle_generic(self, mgr, event)
        }
    }
}

#[layout(single)]
#[handler(msg=u64)]
#[derive(Clone, Debug, Widget)]
struct ComboPopup {
    #[widget_core]
    core: CoreData,
    #[widget]
    inner: MenuFrame<Column<MenuEntry<u64>>>,
}
