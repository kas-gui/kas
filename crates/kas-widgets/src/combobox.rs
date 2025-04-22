// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Combobox

use crate::adapt::AdaptEvents;
use crate::{menu::MenuEntry, Column, Label, Mark};
use kas::event::{Command, FocusSource, ScrollDelta};
use kas::prelude::*;
use kas::theme::FrameStyle;
use kas::theme::{MarkStyle, TextClass};
use kas::Popup;
use std::fmt::Debug;

#[derive(Clone, Debug)]
struct IndexMsg(usize);

impl_scope! {
    /// A pop-up multiple choice menu
    ///
    /// # Messages
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    /// Each choice has an associated value of type `V`.
    ///
    /// If no selection handler exists, then the choice's message is emitted
    /// when selected. If a handler is specified via [`Self::with`] or
    /// [`Self::with_msg`] then this message is passed to the handler and not emitted.
    #[widget {
        layout = frame!(row! [self.label, Mark::new(MarkStyle::Point(Direction::Down))])
            .with_style(FrameStyle::Button)
            .align(AlignHints::CENTER);
        navigable = true;
        hover_highlight = true;
    }]
    pub struct ComboBox<A, V: Clone + Debug + Eq + 'static> {
        core: widget_core!(),
        #[widget(&())]
        label: Label<String>,
        #[widget(&())]
        popup: Popup<AdaptEvents<Column<Vec<MenuEntry<V>>>>>,
        active: usize,
        opening: bool,
        state_fn: Box<dyn Fn(&ConfigCx, &A) -> V>,
        on_select: Option<Box<dyn Fn(&mut EventCx, V)>>,
    }

    impl Tile for Self {
        fn nav_next(&self, _: bool, _: Option<usize>) -> Option<usize> {
            // We have no child within our rect
            None
        }

        fn probe(&self, _: Coord) -> Id {
            self.id()
        }
    }

    impl Events for Self {
        type Data = A;

        fn update(&mut self, cx: &mut ConfigCx, data: &A) {
            let msg = (self.state_fn)(cx, data);
            if let Some(index) = self.popup
                .inner
                .inner
                .iter()
                .enumerate()
                .find_map(|(i, w)| (*w == msg).then_some(i))
            {
                if index != self.active {
                    self.active = index;
                    cx.redraw(&self);
                }
            } else {
                log::warn!("ComboBox::update: unknown entry {msg:?}");
            };
        }

        fn handle_event(&mut self, cx: &mut EventCx, _: &A, event: Event) -> IsUsed {
            let open_popup = |s: &mut Self, cx: &mut EventCx, source: FocusSource| {
                if s.popup.open(cx, &(), s.id()) {
                    if let Some(w) = s.popup.inner.inner.get_child(s.active) {
                        cx.next_nav_focus(w.id(), false, source);
                    }
                }
            };

            match event {
                Event::Command(cmd, code) => {
                    if self.popup.is_open() {
                        let next = |cx: &mut EventCx, clr, rev| {
                            if clr {
                                cx.clear_nav_focus();
                            }
                            cx.next_nav_focus(None, rev, FocusSource::Key);
                        };
                        match cmd {
                            cmd if cmd.is_activate() => {
                                self.popup.close(cx);
                                cx.depress_with_key(self.id(), code);
                            }
                            Command::Up => next(cx, false, true),
                            Command::Down => next(cx, false, false),
                            Command::Home => next(cx, true, false),
                            Command::End => next(cx, true, true),
                            _ => return Unused,
                        }
                    } else {
                        let last = self.len().saturating_sub(1);
                        match cmd {
                            cmd if cmd.is_activate() => {
                                open_popup(self, cx, FocusSource::Key);
                                cx.depress_with_key(self.id(), code);
                            }
                            Command::Up => self.set_active(cx, self.active.saturating_sub(1)),
                            Command::Down => self.set_active(cx, (self.active + 1).min(last)),
                            Command::Home => self.set_active(cx, 0),
                            Command::End => self.set_active(cx, last),
                            _ => return Unused,
                        };
                    }
                    Used
                }
                Event::Scroll(ScrollDelta::Lines(_, y)) if !self.popup.is_open() => {
                    if y > 0.0 {
                        self.set_active(cx, self.active.saturating_sub(1));
                    } else if y < 0.0 {
                        let last = self.len().saturating_sub(1);
                        self.set_active(cx, (self.active + 1).min(last));
                    }
                    Used
                }
                Event::PressStart { press } => {
                    if press.id.as_ref().map(|id| self.is_ancestor_of(id)).unwrap_or(false) {
                        if press.is_primary() {
                            press.grab(self.id(), kas::event::GrabMode::Grab).complete(cx);
                            cx.set_grab_depress(*press, press.id);
                            self.opening = !self.popup.is_open();
                        }
                        Used
                    } else {
                        Unused
                    }
                }
                Event::CursorMove { press } | Event::PressMove { press, .. } => {
                    open_popup(self, cx, FocusSource::Pointer);
                    let cond = self.popup.rect().contains(press.coord);
                    let target = if cond { press.id } else { None };
                    cx.set_grab_depress(press.source, target.clone());
                    if let Some(id) = target {
                        cx.set_nav_focus(id, FocusSource::Pointer);
                    }
                    Used
                }
                Event::PressEnd { press, success } if success => {
                    if let Some(id) = press.id {
                        if self.eq_id(&id) {
                            if self.opening {
                                open_popup(self, cx, FocusSource::Pointer);
                                return Used;
                            }
                        } else if self.popup.is_open() && self.popup.is_ancestor_of(&id) {
                            cx.send_command(id, Command::Activate);
                            return Used;
                        }
                    }
                    self.popup.close(cx);
                    Used
                }
                _ => Unused,
            }
        }

        fn handle_messages(&mut self, cx: &mut EventCx, _: &Self::Data) {
            if let Some(IndexMsg(index)) = cx.try_pop() {
                self.set_active(cx, index);
                self.popup.close(cx);
                if let Some(ref f) = self.on_select {
                    if let Some(msg) = cx.try_pop() {
                        (f)(cx, msg);
                    }
                }
            }
        }
    }
}

impl<A, V: Clone + Debug + Eq + 'static> ComboBox<A, V> {
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
    pub fn new<T, I>(iter: I, state_fn: impl Fn(&ConfigCx, &A) -> V + 'static) -> Self
    where
        T: Into<AccessString>,
        I: IntoIterator<Item = (T, V)>,
    {
        let entries = iter
            .into_iter()
            .map(|(label, msg)| MenuEntry::new_msg(label, msg))
            .collect();
        Self::new_vec(entries, state_fn)
    }

    /// Construct a combobox with the given menu entries
    ///
    /// A combobox presents a menu with a fixed set of choices when clicked.
    ///
    /// The closure `state_fn` selects the active entry from input data.
    pub fn new_vec(
        entries: Vec<MenuEntry<V>>,
        state_fn: impl Fn(&ConfigCx, &A) -> V + 'static,
    ) -> Self {
        let label = entries.first().map(|entry| entry.as_str().to_string());
        let label = Label::new(label.unwrap_or_default()).with_class(TextClass::Button);
        ComboBox {
            core: Default::default(),
            label,
            popup: Popup::new(
                AdaptEvents::new(Column::new(entries)).on_messages(|cx, _, _| {
                    if let Some(_) = cx.try_peek::<V>() {
                        if let Some(index) = cx.last_child() {
                            cx.push(IndexMsg(index));
                        }
                    }
                }),
                Direction::Down,
            ),
            active: 0,
            opening: false,
            state_fn: Box::new(state_fn),
            on_select: None,
        }
    }

    /// Send the message generated by `f` on selection
    #[must_use]
    pub fn with_msg<M: Debug + 'static>(self, f: impl Fn(V) -> M + 'static) -> Self {
        self.with(move |cx, m| cx.push(f(m)))
    }

    /// Call the handler `f` on selection
    ///
    /// On selection of a new choice the closure `f` is called with the choice's
    /// message.
    #[must_use]
    pub fn with<F>(mut self, f: F) -> ComboBox<A, V>
    where
        F: Fn(&mut EventCx, V) + 'static,
    {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Construct a combobox which sends a message on selection
    ///
    /// See [`Self::new`] and [`Self::with_msg`] for documentation.
    pub fn new_msg<T, I, M>(
        iter: I,
        state_fn: impl Fn(&ConfigCx, &A) -> V + 'static,
        msg_fn: impl Fn(V) -> M + 'static,
    ) -> Self
    where
        T: Into<AccessString>,
        I: IntoIterator<Item = (T, V)>,
        M: Debug + 'static,
    {
        Self::new(iter, state_fn).with_msg(msg_fn)
    }
}

impl<A, V: Clone + Debug + Eq + 'static> ComboBox<A, V> {
    /// Get the index of the active choice
    ///
    /// This index is normally less than the number of choices (`self.len()`),
    /// but may not be if set programmatically or there are no choices.
    #[inline]
    pub fn active(&self) -> usize {
        self.active
    }

    /// Set the active choice
    pub fn set_active(&mut self, cx: &mut EventState, index: usize) {
        if self.active != index && index < self.popup.inner.inner.len() {
            self.active = index;
            let string = if index < self.len() {
                self.popup.inner.inner[index].as_str().to_string()
            } else {
                "".to_string()
            };
            self.label.set_string(cx, string);
        }
    }

    /// Get the number of entries
    #[inline]
    pub fn len(&self) -> usize {
        self.popup.inner.inner.len()
    }

    /// True if the box contains no entries
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.popup.inner.inner.is_empty()
    }

    /// Remove all choices
    pub fn clear(&mut self) {
        self.popup.inner.inner.clear()
    }

    /// Add a choice to the combobox, in last position
    ///
    /// Returns the index of the new choice
    //
    // TODO(opt): these methods cause full-window resize. They don't need to
    // resize at all if the menu is closed!
    pub fn push<T: Into<AccessString>>(&mut self, cx: &mut ConfigCx, label: T, msg: V) -> usize {
        let column = &mut self.popup.inner.inner;
        column.push(cx, &(), MenuEntry::new_msg(label, msg))
    }

    /// Pops the last choice from the combobox
    pub fn pop(&mut self, cx: &mut EventState) -> Option<()> {
        self.popup.inner.inner.pop(cx).map(|_| ())
    }

    /// Add a choice at position `index`
    ///
    /// Panics if `index > len`.
    pub fn insert<T: Into<AccessString>>(
        &mut self,
        cx: &mut ConfigCx,
        index: usize,
        label: T,
        msg: V,
    ) {
        let column = &mut self.popup.inner.inner;
        column.insert(cx, &(), index, MenuEntry::new_msg(label, msg));
    }

    /// Removes the choice at position `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, cx: &mut EventState, index: usize) {
        self.popup.inner.inner.remove(cx, index);
    }

    /// Replace the choice at `index`
    ///
    /// Panics if `index` is out of bounds.
    pub fn replace<T: Into<AccessString>>(
        &mut self,
        cx: &mut ConfigCx,
        index: usize,
        label: T,
        msg: V,
    ) {
        self.popup
            .inner
            .inner
            .replace(cx, &(), index, MenuEntry::new_msg(label, msg));
    }
}
