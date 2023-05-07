// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use super::{menu::MenuEntry, Column, Mark, PopupFrame, StringLabel};
use kas::event::{Command, Scroll, ScrollDelta};
use kas::prelude::*;
use kas::theme::{MarkStyle, TextClass};
use kas::WindowId;
use std::fmt::Debug;

#[derive(Clone, Debug)]
struct IndexMsg(usize);

impl_scope! {
    /// A pop-up multiple choice menu
    ///
    /// # Messages
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    /// Each choice has an associated "message" value of type `M`.
    ///
    /// If no selection handler exists, then the choice's message is emitted
    /// when selected. If a handler is specified via [`Self::on_select`], then
    /// this message is passed to the handler and not emitted.
    #[autoimpl(Debug ignore self.on_select)]
    #[impl_default]
    #[widget {
        layout = button 'frame: row: [self.label, self.mark];
        navigable = true;
        hover_highlight = true;
    }]
    pub struct ComboBox<M: Clone + Debug + 'static> {
        core: widget_core!(),
        #[widget]
        label: StringLabel,
        #[widget]
        mark: Mark = Mark::new(MarkStyle::Point(Direction::Down)),
        #[widget]
        popup: ComboPopup<M>,
        active: usize,
        opening: bool,
        popup_id: Option<WindowId>,
        on_select: Option<Box<dyn Fn(&mut EventCx<()>, M)>>,
    }

    impl Widget for Self {
        fn pre_configure(&mut self, mgr: &mut ConfigCx<Self::Data>, id: WidgetId) {
            self.core.id = id;
            mgr.new_accel_layer(self.id(), true);
        }

        fn nav_next(&mut self, _: &mut EventCx<Self::Data>, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn handle_event(&mut self, mgr: &mut EventCx<Self::Data>, event: Event) -> Response {
            let open_popup = |s: &mut Self, mgr: &mut EventCx<Self::Data>, key_focus: bool| {
                s.popup_id = mgr.add_popup(kas::Popup {
                    id: s.popup.id(),
                    parent: s.id(),
                    direction: Direction::Down,
                });
                s.popup.inner.inner.for_child(mgr.data(), s.active, |w| {
                    mgr.next_nav_focus(w.id(), false, key_focus);
                });
            };

            match event {
                Event::Command(cmd) => {
                    if let Some(popup_id) = self.popup_id {
                        let next = |mgr: &mut EventCx<Self::Data>, id, clr, rev| {
                            if clr {
                                mgr.clear_nav_focus();
                            }
                            mgr.next_nav_focus(Some(id), rev, true);
                        };
                        match cmd {
                            cmd if cmd.is_activate() => mgr.close_window(popup_id, true),
                            Command::Up => next(mgr, self.id(), false, true),
                            Command::Down => next(mgr, self.id(), false, false),
                            Command::Home => next(mgr, self.id(), true, false),
                            Command::End => next(mgr, self.id(), true, true),
                            _ => return Response::Unused,
                        }
                    } else {
                        let last = self.len().saturating_sub(1);
                        match cmd {
                            cmd if cmd.is_activate() => open_popup(self, mgr, true),
                            Command::Up => *mgr |= self.set_active(self.active.saturating_sub(1)),
                            Command::Down => *mgr |= self.set_active((self.active + 1).min(last)),
                            Command::Home => *mgr |= self.set_active(0),
                            Command::End => *mgr |= self.set_active(last),
                            _ => return Response::Unused,
                        }
                    }
                    Response::Used
                }
                Event::Scroll(ScrollDelta::LineDelta(_, y)) if self.popup_id.is_none() => {
                    if y > 0.0 {
                        *mgr |= self.set_active(self.active.saturating_sub(1));
                    } else if y < 0.0 {
                        let last = self.len().saturating_sub(1);
                        *mgr |= self.set_active((self.active + 1).min(last));
                    }
                    Response::Used
                }
                Event::PressStart { press } => {
                    if press.id.as_ref().map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                        if press.is_primary() {
                            press.grab(self.id()).with_cx(mgr);
                            mgr.set_grab_depress(*press, press.id);
                            self.opening = self.popup_id.is_none();
                        }
                        Response::Used
                    } else {
                        if let Some(id) = self.popup_id {
                            mgr.close_window(id, false);
                        }
                        Response::Unused
                    }
                }
                Event::PressMove { press, .. } => {
                    if self.popup_id.is_none() {
                        open_popup(self, mgr, false);
                    }
                    let cond = self.popup.inner.rect().contains(press.coord);
                    let target = if cond { press.id } else { None };
                    mgr.set_grab_depress(press.source, target.clone());
                    if let Some(id) = target {
                        mgr.set_nav_focus(id, false);
                    }
                    Response::Used
                }
                Event::PressEnd { press, success } if success => {
                    if let Some(id) = press.id {
                        if self.eq_id(&id) {
                            if self.opening {
                                if self.popup_id.is_none() {
                                    open_popup(self, mgr, false);
                                }
                                return Response::Used;
                            }
                        } else if self.popup_id.is_some() && self.popup.is_ancestor_of(&id) {
                            mgr.send(self, id, Event::Command(Command::Activate));
                            return Response::Used;
                        }
                    }
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id, true);
                    }
                    Response::Used
                }
                Event::PressEnd { .. } => Response::Used,
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventCx<Self::Data>) {
            if let Some(IndexMsg(index)) = mgr.try_pop() {
                *mgr |= self.set_active(index);
                if let Some(id) = self.popup_id {
                    mgr.close_window(id, true);
                }
                if let Some(ref f) = self.on_select {
                    if let Some(msg) = mgr.try_pop() {
                        (f)(mgr, msg);
                    }
                }
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventCx<Self::Data>, _: Scroll) {
            mgr.set_scroll(Scroll::None);
        }
    }
}

impl<M, T, I> From<I> for ComboBox<M>
where
    M: Clone + Debug + 'static,
    T: Into<AccelString>,
    I: IntoIterator<Item = (T, M)>,
{
    /// Construct a combobox
    ///
    /// Constructs a combobox with labels derived from an iterator over string
    /// types. For example:
    /// ```
    /// # use kas_widgets::ComboBox;
    /// let combobox = ComboBox::from([("zero", 0), ("one", 1), ("two", 2)]);
    /// ```
    ///
    /// Initially, the first entry is active.
    #[inline]
    fn from(iter: I) -> Self {
        let entries = iter
            .into_iter()
            .map(|(label, msg)| MenuEntry::new(label, msg))
            .collect();
        Self::new_vec(entries)
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    /// Construct an empty combobox
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a combobox with the given menu entries
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    ///
    /// Initially, the first entry is active.
    #[inline]
    pub fn new_vec(entries: Vec<MenuEntry<M>>) -> Self {
        let label = entries.get(0).map(|entry| entry.get_string());
        let label = StringLabel::new(label.unwrap_or_default()).with_class(TextClass::Button);
        ComboBox {
            label,
            popup: ComboPopup {
                core: Default::default(),
                inner: PopupFrame::new(
                    Column::new_vec(entries).on_message(|mgr, index| mgr.push(IndexMsg(index))),
                ),
            },
            ..Default::default()
        }
    }

    /// Set the selection handler `f`
    ///
    /// On selection of a new choice the closure `f` is called with the choice's
    /// message.
    #[inline]
    #[must_use]
    pub fn on_select<F>(self, f: F) -> ComboBox<M>
    where
        F: Fn(&mut EventCx<()>, M) + 'static,
    {
        ComboBox {
            core: self.core,
            label: self.label,
            mark: self.mark,
            popup: self.popup,
            active: self.active,
            opening: self.opening,
            popup_id: self.popup_id,
            on_select: Some(Box::new(f)),
        }
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    /// Get the index of the active choice
    ///
    /// This index is normally less than the number of choices (`self.len()`),
    /// but may not be if set programmatically or there are no choices.
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active choice (inline style)
    #[inline]
    pub fn with_active(mut self, index: usize) -> Self {
        let _ = self.set_active(index);
        self
    }

    /// Set the active choice
    pub fn set_active(&mut self, index: usize) -> Action {
        if self.active != index && index < self.popup.inner.len() {
            self.active = index;
            let string = if index < self.len() {
                self.popup.inner[index].get_string()
            } else {
                "".to_string()
            };
            self.label.set_string(string)
        } else {
            Action::empty()
        }
    }

    /// Get the number of entries
    #[inline]
    pub fn len(&self) -> usize {
        self.popup.inner.len()
    }

    /// True if the box contains no entries
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.popup.inner.is_empty()
    }

    /// Remove all choices
    pub fn clear(&mut self) {
        self.popup.inner.clear()
    }

    /// Add a choice to the combobox, in last position
    ///
    /// Returns the index of the new choice
    //
    // TODO(opt): these methods cause full-window resize. They don't need to
    // resize at all if the menu is closed!
    pub fn push<T: Into<AccelString>>(&mut self, mgr: &mut EventState, label: T, msg: M) -> usize {
        let column = &mut self.popup.inner;
        column.push(mgr, MenuEntry::new(label, msg))
    }

    /// Pops the last choice from the combobox
    pub fn pop(&mut self, mgr: &mut EventState) -> Option<()> {
        self.popup.inner.pop(mgr).map(|_| ())
    }

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert<T: Into<AccelString>>(
        &mut self,
        mgr: &mut EventState,
        index: usize,
        label: T,
        msg: M,
    ) {
        let column = &mut self.popup.inner;
        column.insert(mgr, index, MenuEntry::new(label, msg));
    }

    /// Removes the choice at position `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, mgr: &mut EventState, index: usize) {
        self.popup.inner.remove(mgr, index);
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<AccelString>>(
        &mut self,
        mgr: &mut EventState,
        index: usize,
        label: T,
        msg: M,
    ) {
        self.popup
            .inner
            .replace(mgr, index, MenuEntry::new(label, msg));
    }
}

impl_scope! {
    #[autoimpl(Default)]
    #[derive(Clone, Debug)]
    #[widget{
        layout = self.inner;
    }]
    struct ComboPopup<M: Clone + Debug + 'static> {
        core: widget_core!(),
        #[widget]
        inner: PopupFrame<Column<MenuEntry<M>>>,
    }
}
