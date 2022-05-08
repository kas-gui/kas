// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use super::{menu::MenuEntry, Column, PopupFrame};
use kas::component::{Label, Mark};
use kas::event::{Command, Scroll};
use kas::layout;
use kas::prelude::*;
use kas::theme::{MarkStyle, TextClass};
use kas::WindowId;
use std::fmt::Debug;
use std::rc::Rc;

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
    #[derive(Clone)]
    #[widget {
        layout = button: row: [self.label, self.mark];
    }]
    pub struct ComboBox<M: Clone + Debug + 'static> {
        core: widget_core!(),
        label: Label<String>,
        mark: Mark,
        layout_list: layout::FixedRowStorage<2>,
        layout_frame: layout::FrameStorage,
        #[widget]
        popup: ComboPopup<M>,
        active: usize,
        opening: bool,
        popup_id: Option<WindowId>,
        on_select: Option<Rc<dyn Fn(&mut EventMgr, M)>>,
    }

    impl Widget for Self {
        fn pre_configure(&mut self, mgr: &mut SetRectMgr, id: WidgetId) {
            self.core.id = id;
            mgr.new_accel_layer(self.id(), true);
        }

        fn key_nav(&self) -> bool {
            true
        }

        fn hover_highlight(&self) -> bool {
            true
        }

        fn spatial_nav(&mut self, _: &mut SetRectMgr, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn handle_event(&mut self, mgr: &mut EventMgr, event: Event) -> Response {
            let open_popup = |s: &mut Self, mgr: &mut EventMgr, key_focus: bool| {
                s.popup_id = mgr.add_popup(kas::Popup {
                    id: s.popup.id(),
                    parent: s.id(),
                    direction: Direction::Down,
                });
                if let Some(id) = s.popup.inner.inner.get_child(s.active).map(|w| w.id()) {
                    mgr.set_nav_focus(id, key_focus);
                }
            };
            match event {
                Event::Activate => {
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id, true);
                    } else {
                        open_popup(self, mgr, true);
                    }
                    Response::Used
                }
                Event::Command(cmd, _) => {
                    let next = |mgr: &mut EventMgr, s, clr, rev| {
                        if clr {
                            mgr.clear_nav_focus();
                        }
                        mgr.next_nav_focus(s, rev, true);
                        Response::Used
                    };
                    match cmd {
                        Command::Up => next(mgr, self, false, true),
                        Command::Down => next(mgr, self, false, false),
                        Command::Home => next(mgr, self, true, false),
                        Command::End => next(mgr, self, true, true),
                        _ => Response::Unused,
                    }
                }
                Event::PressStart {
                    source,
                    start_id,
                    coord,
                } => {
                    if start_id.as_ref().map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                        if source.is_primary() {
                            mgr.grab_press_unique(self.id(), source, coord, None);
                            mgr.set_grab_depress(source, start_id);
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
                Event::PressMove {
                    source,
                    cur_id,
                    coord,
                    ..
                } => {
                    if self.popup_id.is_none() {
                        open_popup(self, mgr, false);
                    }
                    let cond = self.popup.inner.rect().contains(coord);
                    let target = if cond { cur_id } else { None };
                    mgr.set_grab_depress(source, target.clone());
                    if let Some(id) = target {
                        mgr.set_nav_focus(id, false);
                    }
                    Response::Used
                }
                Event::PressEnd { ref end_id, success, .. } if success => {
                    if let Some(ref id) = end_id {
                        if self.eq_id(id) {
                            if self.opening {
                                if self.popup_id.is_none() {
                                    open_popup(self, mgr, false);
                                }
                                return Response::Used;
                            }
                        } else if self.popup_id.is_some() && self.popup.is_ancestor_of(id) {
                            return mgr.send(self, id.clone(), Event::Activate);
                        }
                    }
                    if let Some(id) = self.popup_id {
                        mgr.close_window(id, true);
                    }
                    Response::Used
                }
                Event::PressEnd { .. } =>Response::Used,
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_message(&mut self, mgr: &mut EventMgr, _: usize) {
            if let Some(IndexMsg(index)) = mgr.try_pop_msg() {
                *mgr |= self.set_active(index);
                if let Some(id) = self.popup_id {
                    mgr.close_window(id, true);
                }
                if let Some(ref f) = self.on_select {
                    if let Some(msg) = mgr.try_pop_msg() {
                        (f)(mgr, msg);
                    }
                }
            }
        }

        fn handle_scroll(&mut self, mgr: &mut EventMgr, _: Scroll) {
            mgr.set_scroll(Scroll::None);
        }
    }
}

impl<M: Clone + Debug + 'static> ComboBox<M> {
    /// Construct a combobox
    ///
    /// Constructs a combobox with labels derived from an iterator over string
    /// types. For example:
    /// ```
    /// # use kas_widgets::ComboBox;
    /// let combobox = ComboBox::new_from_iter([("zero", 0), ("one", 1), ("two", 2)].into_iter());
    /// ```
    ///
    /// Initially, the first entry is active.
    #[inline]
    pub fn new_from_iter<T: Into<AccelString>, I: IntoIterator<Item = (T, M)>>(iter: I) -> Self {
        let entries = iter
            .into_iter()
            .map(|(label, msg)| MenuEntry::new(label, msg))
            .collect();
        Self::new(entries)
    }

    /// Construct a combobox with the given menu entries
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    ///
    /// Initially, the first entry is active.
    #[inline]
    pub fn new(entries: Vec<MenuEntry<M>>) -> Self {
        let label = entries.get(0).map(|entry| entry.get_string());
        let label = Label::new(label.unwrap_or("".to_string()), TextClass::Button);
        ComboBox {
            core: Default::default(),
            label,
            mark: Mark::new(MarkStyle::Point(Direction::Down)),
            layout_list: Default::default(),
            layout_frame: Default::default(),
            popup: ComboPopup {
                core: Default::default(),
                inner: PopupFrame::new(
                    Column::new(entries).on_message(|mgr, index| mgr.push_msg(IndexMsg(index))),
                ),
            },
            active: 0,
            opening: false,
            popup_id: None,
            on_select: None,
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
        F: Fn(&mut EventMgr, M) + 'static,
    {
        ComboBox {
            core: self.core,
            label: self.label,
            mark: self.mark,
            layout_list: self.layout_list,
            layout_frame: self.layout_frame,
            popup: self.popup,
            active: self.active,
            opening: self.opening,
            popup_id: self.popup_id,
            on_select: Some(Rc::new(f)),
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
    pub fn set_active(&mut self, index: usize) -> TkAction {
        if self.active != index && index < self.popup.inner.len() {
            self.active = index;
            let string = if index < self.len() {
                self.popup.inner[index].get_string()
            } else {
                "".to_string()
            };
            let avail = self.core.rect.size.clamped_sub(self.layout_frame.size);
            self.label.set_text_and_prepare(string, avail)
        } else {
            TkAction::empty()
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
    pub fn push<T: Into<AccelString>>(&mut self, mgr: &mut SetRectMgr, label: T, msg: M) -> usize {
        let column = &mut self.popup.inner;
        column.push(mgr, MenuEntry::new(label, msg))
    }

    /// Pops the last choice from the combobox
    pub fn pop(&mut self, mgr: &mut SetRectMgr) -> Option<()> {
        self.popup.inner.pop(mgr).map(|_| ())
    }

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert<T: Into<AccelString>>(
        &mut self,
        mgr: &mut SetRectMgr,
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
    pub fn remove(&mut self, mgr: &mut SetRectMgr, index: usize) {
        self.popup.inner.remove(mgr, index);
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<AccelString>>(
        &mut self,
        mgr: &mut SetRectMgr,
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
