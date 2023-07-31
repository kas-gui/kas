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
    #[widget {
        layout = button! 'frame(row! [self.label, self.mark]);
        navigable = true;
        hover_highlight = true;
    }]
    pub struct ComboBox<A, M: Clone + Debug + Eq + 'static> {
        core: widget_core!(),
        #[widget(&())]
        label: StringLabel,
        #[widget(&())]
        mark: Mark,
        #[widget(&())]
        popup: ComboPopup<M>,
        active: usize,
        opening: bool,
        popup_id: Option<WindowId>,
        state_fn: Box<dyn Fn(&ConfigCx, &A) -> M>,
        on_select: Option<Box<dyn Fn(&mut EventCx, M)>>,
    }

    impl Layout for Self {
        fn nav_next(&self, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }
    }

    impl Events for Self {
        type Data = A;

        fn pre_configure(&mut self, cx: &mut ConfigCx, id: WidgetId) {
            self.core.id = id;
            cx.new_accel_layer(self.id(), true);
        }

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let msg = (self.state_fn)(cx, data);
            let index = 'outer: {
                for (i, w) in self.popup.inner.iter().enumerate() {
                    if *w == msg {
                        break 'outer i;
                    }
                }

                log::warn!("ComboBox::update: unknown entry {msg:?}");
                return;
            };
            if index != self.active {
                self.active = index;
                *cx |= Action::REDRAW;
            }
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &A, event: Event) -> Response {
            let open_popup = |s: &mut Self, cx: &mut EventCx, key_focus: bool| {
                s.popup_id = cx.add_popup(kas::Popup {
                    id: s.popup.id(),
                    parent: s.id(),
                    direction: Direction::Down,
                });
                if let Some(w) = s.popup.inner.inner.get_child(s.active) {
                    cx.next_nav_focus(w.id(), false, key_focus);
                }
            };

            match event {
                Event::Command(cmd) => {
                    if let Some(popup_id) = self.popup_id {
                        let next = |cx: &mut EventCx, id, clr, rev| {
                            if clr {
                                cx.clear_nav_focus();
                            }
                            cx.next_nav_focus(Some(id), rev, true);
                        };
                        match cmd {
                            cmd if cmd.is_activate() => cx.close_window(popup_id, true),
                            Command::Up => next(cx, self.id(), false, true),
                            Command::Down => next(cx, self.id(), false, false),
                            Command::Home => next(cx, self.id(), true, false),
                            Command::End => next(cx, self.id(), true, true),
                            _ => return Response::Unused,
                        }
                    } else {
                        let last = self.len().saturating_sub(1);
                        match cmd {
                            cmd if cmd.is_activate() => open_popup(self, cx, true),
                            Command::Up => *cx |= self.set_active(self.active.saturating_sub(1)),
                            Command::Down => *cx |= self.set_active((self.active + 1).min(last)),
                            Command::Home => *cx |= self.set_active(0),
                            Command::End => *cx |= self.set_active(last),
                            _ => return Response::Unused,
                        }
                    }
                    Response::Used
                }
                Event::Scroll(ScrollDelta::LineDelta(_, y)) if self.popup_id.is_none() => {
                    if y > 0.0 {
                        *cx |= self.set_active(self.active.saturating_sub(1));
                    } else if y < 0.0 {
                        let last = self.len().saturating_sub(1);
                        *cx |= self.set_active((self.active + 1).min(last));
                    }
                    Response::Used
                }
                Event::PressStart { press } => {
                    if press.id.as_ref().map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                        if press.is_primary() {
                            press.grab(self.id()).with_cx(cx);
                            cx.set_grab_depress(*press, press.id);
                            self.opening = self.popup_id.is_none();
                        }
                        Response::Used
                    } else {
                        if let Some(id) = self.popup_id {
                            cx.close_window(id, false);
                        }
                        Response::Unused
                    }
                }
                Event::CursorMove { press } | Event::PressMove { press, .. } => {
                    if self.popup_id.is_none() {
                        open_popup(self, cx, false);
                    }
                    let cond = self.popup.inner.rect().contains(press.coord);
                    let target = if cond { press.id } else { None };
                    cx.set_grab_depress(press.source, target.clone());
                    if let Some(id) = target {
                        cx.set_nav_focus(id, false);
                    }
                    Response::Used
                }
                Event::PressEnd { press, success } if success => {
                    if let Some(id) = press.id {
                        if self.eq_id(&id) {
                            if self.opening {
                                if self.popup_id.is_none() {
                                    open_popup(self, cx, false);
                                }
                                return Response::Used;
                            }
                        } else if self.popup_id.is_some() && self.popup.is_ancestor_of(&id) {
                            cx.send(id, Event::Command(Command::Activate));
                            return Response::Used;
                        }
                    }
                    if let Some(id) = self.popup_id {
                        cx.close_window(id, true);
                    }
                    Response::Used
                }
                Event::PopupRemoved(id) => {
                    debug_assert_eq!(Some(id), self.popup_id);
                    self.popup_id = None;
                    Response::Used
                }
                _ => Response::Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(IndexMsg(index)) = cx.try_pop() {
                *cx |= self.set_active(index);
                if let Some(id) = self.popup_id {
                    cx.close_window(id, true);
                }
                if let Some(ref f) = self.on_select {
                    if let Some(msg) = cx.try_pop() {
                        (f)(cx, msg);
                    }
                }
            }
        }

        fn handle_scroll(&mut self, cx: &mut EventCx, _: &Self::Data, _: Scroll) {
            cx.set_scroll(Scroll::None);
        }
    }
}

impl<A, M: Clone + Debug + Eq + 'static> ComboBox<A, M> {
    /// Construct a combobox
    ///
    /// Constructs a combobox with labels derived from an iterator over string
    /// types. For example:
    /// ```
    /// # use kas_widgets::ComboBox;
    /// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    /// enum Select { A, B, C }
    ///
    /// let combobox = ComboBox::new(
    ///     [("A", Select::A), ("B", Select::B), ("C", Select::C)],
    ///     |_, selection| *selection,
    /// );
    /// ```
    ///
    /// The closure `state_fn` selects the active entry from input data.
    #[inline]
    pub fn new<T, I>(iter: I, state_fn: impl Fn(&ConfigCx, &A) -> M + 'static) -> Self
    where
        T: Into<AccelString>,
        I: IntoIterator<Item = (T, M)>,
    {
        let entries = iter
            .into_iter()
            .map(|(label, msg)| MenuEntry::new(label, msg))
            .collect();
        Self::new_vec(entries, state_fn)
    }

    /// Construct a combobox with the given menu entries
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    ///
    /// The closure `state_fn` selects the active entry from input data.
    #[inline]
    pub fn new_vec(
        entries: Vec<MenuEntry<M>>,
        state_fn: impl Fn(&ConfigCx, &A) -> M + 'static,
    ) -> Self {
        let label = entries.get(0).map(|entry| entry.get_string());
        let label = StringLabel::new(label.unwrap_or_default()).with_class(TextClass::Button);
        ComboBox {
            core: Default::default(),
            label,
            mark: Mark::new(MarkStyle::Point(Direction::Down)),
            popup: ComboPopup {
                core: Default::default(),
                inner: PopupFrame::new(
                    Column::new_vec(entries).on_messages(|cx, index| cx.push(IndexMsg(index))),
                ),
            },
            active: 0,
            opening: false,
            popup_id: None,
            state_fn: Box::new(state_fn),
            on_select: None,
        }
    }

    /// Send a message on selection
    #[inline]
    #[must_use]
    pub fn msg_on_select<M2: Debug + 'static>(self, f: impl Fn(M) -> M2 + 'static) -> Self {
        self.on_select(move |cx, m| cx.push(f(m)))
    }

    /// Set the selection handler `f`
    ///
    /// On selection of a new choice the closure `f` is called with the choice's
    /// message.
    #[inline]
    #[must_use]
    pub fn on_select<F>(mut self, f: F) -> ComboBox<A, M>
    where
        F: Fn(&mut EventCx, M) + 'static,
    {
        self.on_select = Some(Box::new(f));
        self
    }
}

impl<A, M: Clone + Debug + Eq + 'static> ComboBox<A, M> {
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
    pub fn push<T: Into<AccelString>>(&mut self, cx: &mut ConfigCx, label: T, msg: M) -> usize {
        let column = &mut self.popup.inner;
        column.push(cx, &(), MenuEntry::new(label, msg))
    }

    /// Pops the last choice from the combobox
    pub fn pop(&mut self, cx: &mut EventState) -> Option<()> {
        self.popup.inner.pop(cx).map(|_| ())
    }

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert<T: Into<AccelString>>(
        &mut self,
        cx: &mut ConfigCx,
        index: usize,
        label: T,
        msg: M,
    ) {
        let column = &mut self.popup.inner;
        column.insert(cx, &(), index, MenuEntry::new(label, msg));
    }

    /// Removes the choice at position `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, cx: &mut EventState, index: usize) {
        self.popup.inner.remove(cx, index);
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<AccelString>>(
        &mut self,
        cx: &mut ConfigCx,
        index: usize,
        label: T,
        msg: M,
    ) {
        self.popup
            .inner
            .replace(cx, &(), index, MenuEntry::new(label, msg));
    }
}

impl_scope! {
    #[autoimpl(Default)]
    #[widget{
        Data = ();
        layout = self.inner;
    }]
    struct ComboPopup<M: Clone + Debug + 'static> {
        core: widget_core!(),
        #[widget]
        inner: PopupFrame<Column<MenuEntry<M>>>,
    }
}
